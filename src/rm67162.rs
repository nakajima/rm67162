extern crate esp_hal;

use crate::orientation::Orientation;
use embedded_graphics::framebuffer::Framebuffer;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::raw::BigEndian;
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::raw::ToBytes;
use embedded_graphics::pixelcolor::Rgb565;
use esp_backtrace as _;
use esp_hal::delay::Delay;
use esp_hal::gpio::AnyPin;
use esp_hal::gpio::Output;
use esp_hal::spi::master::{Address, Command, HalfDuplexReadWrite, SpiDmaBus};
use esp_hal::spi::{HalfDuplexMode, SpiDataMode};
use esp_hal::Async;
use esp_println::println;

pub const BUFFER_PIXELS: usize = 16368 / 2;
pub const BUFFER_SIZE: usize = BUFFER_PIXELS * 2;

pub struct RM67162<'d> {
    pub orientation: Orientation,
    pub(crate) spi: SpiDmaBus<'d, HalfDuplexMode, Async>,
    pub(crate) chip_select: Output<'d>,
    delay: Delay,
}

impl<'d> RM67162<'d> {
    pub fn new(
        spi: SpiDmaBus<'d, HalfDuplexMode, Async>,
        chip_select: Output<'d, AnyPin>,
        delay: Delay,
        orientation: Orientation,
    ) -> Self {
        Self {
            spi,
            orientation,
            chip_select,
            delay,
        }
    }

    pub fn reset(&mut self, reset: &mut Output<'d, AnyPin>) {
        reset.set_low();
        self.delay.delay_millis(300);
        reset.set_high();
        self.delay.delay_millis(200);
        println!("display reset!")
    }

    pub fn initialize(&mut self) {
        // Sleep out
        for _ in 0..3 {
            self.command(0x11, &[]).unwrap();
            self.delay.delay_millis(120);

            self.command(0x3A, &[0x55]).unwrap(); // 16 bit color mode
            self.command(0x51, &[0x00]).unwrap(); // write brightness

            self.command(0x29, &[]).unwrap(); // display on
            self.delay.delay_millis(10);

            self.command(0x51, &[0xE0]).unwrap(); // write brightness
        }

        self.command(0x36, &[self.orientation.to_madctr()]).unwrap();
    }

    pub fn set_orientation(&mut self, orientation: Orientation) -> Result<(), esp_hal::spi::Error> {
        self.orientation = orientation;
        self.command(0x36, &[self.orientation.to_madctr()])
    }

    pub(crate) fn set_address(
        &mut self,
        x1: u16,
        y1: u16,
        x2: u16,
        y2: u16,
    ) -> Result<(), esp_hal::spi::Error> {
        self.command(
            0x2a,
            &[
                (x1 >> 8) as u8,
                (x1 & 0xFF) as u8,
                (x2 >> 8) as u8,
                (x2 & 0xFF) as u8,
            ],
        )
        .unwrap();

        self.command(
            0x2b,
            &[
                (y1 >> 8) as u8,
                (y1 & 0xFF) as u8,
                (y2 >> 8) as u8,
                (y2 & 0xFF) as u8,
            ],
        )
        .unwrap();

        self.command(0x2c, &[])
    }

    pub fn send_chunk(&mut self, chunk: *const u8, len: usize, is_first: bool) {
        let data = core::ptr::slice_from_raw_parts(chunk, len);

        assert!(
            self.chip_select.is_set_low(),
            "attempting to send chunk while chip select is high"
        );

        if is_first {
            self.spi
                .write(
                    SpiDataMode::Quad,
                    Command::Command8(0x32, SpiDataMode::Single),
                    Address::Address24(0x2C << 8, SpiDataMode::Single),
                    0,
                    unsafe { &*data },
                )
                .unwrap();
        } else {
            self.spi
                .write(SpiDataMode::Quad, Command::None, Address::None, 0, unsafe {
                    &*data
                })
                .unwrap();
        }
    }

    pub fn fill_with<const W: usize, const H: usize>(
        &mut self,
        frame_buffer: &Framebuffer<
            Rgb565,
            RawU16,
            BigEndian,
            W,
            H,
            { embedded_graphics::framebuffer::buffer_size::<Rgb565>(536, 240) },
        >,
    ) -> Result<(), esp_hal::spi::Error> {
        self.set_address(
            0,
            0,
            self.size().width as u16 - 1,
            self.size().height as u16 - 1,
        )?;

        let mut is_first = true;

        self.chip_select.set_low();
        for chunk in frame_buffer.data().chunks_exact(BUFFER_SIZE) {
            self.send_chunk(chunk.as_ptr(), chunk.len(), is_first);
            is_first = false;
        }
        self.chip_select.set_high();

        Ok(())
    }

    pub(crate) fn draw_point(
        &mut self,
        x: u16,
        y: u16,
        color: Rgb565,
    ) -> Result<(), esp_hal::spi::Error> {
        self.set_address(x, y, x, y).unwrap();

        self.chip_select.set_low();
        self.spi
            .write(
                SpiDataMode::Quad,
                Command::Command8(0x32, SpiDataMode::Single),
                Address::Address24(0x2C << 8, SpiDataMode::Single),
                0,
                &color.to_be_bytes(),
            )
            .unwrap();
        self.chip_select.set_high();

        Ok(())
    }

    fn command(&mut self, cmd: u32, parameters: &[u8]) -> Result<(), esp_hal::spi::Error> {
        self.chip_select.set_low();
        self.spi
            .write(
                SpiDataMode::Single,
                Command::Command8(0x02, SpiDataMode::Single),
                Address::Address24(cmd << 8, SpiDataMode::Single),
                0,
                &parameters,
            )
            .unwrap();
        self.chip_select.set_high();
        Ok(())
    }

    pub fn version(&mut self) -> Result<[u8; 3], esp_hal::spi::Error> {
        self.chip_select.set_low();

        let mut buf: [u8; 3] = [0xFF, 0xFF, 0xFF];
        self.spi
            .read(
                SpiDataMode::Quad,
                Command::None,
                Address::Address24(0xDA, SpiDataMode::Quad),
                0,
                &mut buf,
            )
            .unwrap();

        self.chip_select.set_high();

        Ok(buf)
    }
}

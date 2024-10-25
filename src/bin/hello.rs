#![no_std]
#![no_main]

extern crate alloc;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::framebuffer::Framebuffer;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::raw::BigEndian;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{Point, RgbColor, WebColors};
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::Drawable;
use embedded_vintage_fonts::FONT_24X32;
use esp_backtrace as _;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, NoPin, Output};
use esp_hal::rtc_cntl::Rtc;
use esp_hal::spi::SpiMode;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{
    dma::{Dma, DmaPriority, DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::Io,
    prelude::*,
    spi::master::Spi,
};
use esp_println::println;
use rm67162::{
    orientation::Orientation,
    rm67162::{BUFFER_SIZE, RM67162},
};

#[entry]
fn main() -> ! {
    esp_alloc::heap_allocator!(32 * 1024);

    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.LPWR);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1);
    let mut wdt1 = timer_group1.wdt;
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();
    println!("Hello world!");

    let delay = Delay::new();

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // This needs to be high for some reason.
    _ = Output::new_typed(io.pins.gpio38, Level::High);

    let mut reset = Output::new(io.pins.gpio17, Level::High);
    let chip_select = Output::new(io.pins.gpio6, Level::High);

    let sclk = io.pins.gpio47;
    let d0 = io.pins.gpio18;
    let d1 = io.pins.gpio7;
    let d2 = io.pins.gpio48;
    let d3 = io.pins.gpio5;

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(BUFFER_SIZE);
    let rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let spi = Spi::new(peripherals.SPI2, 75_u32.MHz(), SpiMode::Mode0)
        .with_sck(sclk)
        .with_mosi(d0)
        .with_miso(d1)
        .with_sio2(d2)
        .with_sio3(d3)
        .with_dma(dma_channel.configure_for_async(false, DmaPriority::Priority0))
        .with_buffers(rx_buf, tx_buf);

    let mut display = RM67162::new(spi, chip_select, delay, Orientation::PortraitFlipped);
    display.reset(&mut reset);
    display.initialize();
    println!("screen init ok");

    let mut frame_buffer = Framebuffer::<
        Rgb565,
        _,
        BigEndian,
        240,
        536,
        { embedded_graphics::framebuffer::buffer_size::<Rgb565>(240, 536) },
    >::new();

    frame_buffer.clear(Rgb565::BLUE).unwrap();
    display.fill_with(&frame_buffer).unwrap();

    loop {
        let character_style = MonoTextStyle::new(&FONT_24X32, Rgb565::CSS_AQUA);
        Text::with_alignment(
            "hello\nworld",
            Point::new(80, 40),
            character_style,
            Alignment::Center,
        )
        .draw(&mut frame_buffer)
        .unwrap();
        println!("drew text");

        display.fill_with(&frame_buffer).unwrap();

        delay.delay_millis(20);
    }
}

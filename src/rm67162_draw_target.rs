use embedded_graphics::pixelcolor::raw::ToBytes;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, OriginDimensions, Size},
    Pixel,
};

use crate::{
    orientation::Orientation,
    rm67162::{BUFFER_PIXELS, RM67162},
};

impl<'d> OriginDimensions for RM67162<'d> {
    /// Returns the bounding box.
    fn size(&self) -> Size {
        if matches!(
            self.orientation,
            Orientation::Landscape | Orientation::LandscapeFlipped
        ) {
            Size::new(536, 240)
        } else {
            Size::new(240, 536)
        }
    }
}

impl<'d> DrawTarget for RM67162<'d> {
    type Color = Rgb565;

    type Error = esp_hal::spi::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for Pixel(pt, color) in pixels {
            if pt.x < 0 || pt.y < 0 {
                continue;
            }

            self.draw_point(pt.x as u16, pt.y as u16, color).unwrap();
        }

        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.set_address(
            area.top_left.x as u16,
            area.top_left.y as u16,
            area.size.width as u16,
            area.size.height as u16,
        )?;

        let mut first_send = true;
        self.chip_select.set_low();

        for color in colors
            .into_iter()
            .take(area.size.width as usize * area.size.height as usize)
        {
            let bytes = color.to_be_bytes();
            self.send_chunk(bytes.as_ptr(), bytes.len(), first_send);
            first_send = false;
        }

        self.chip_select.set_high();

        Ok(())
    }
}

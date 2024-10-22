use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, OriginDimensions, Size},
    Pixel,
};

use crate::{orientation::Orientation, rm67162::RM67162};

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
}

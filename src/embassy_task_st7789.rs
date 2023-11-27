use display_interface_spi::SPIInterfaceNoCS;
use display_interface::DisplayError;
use hal::{
    peripherals::SPI2,
    gpio::{PushPull, Output, GpioPin},
    spi::FullDuplexMode,
    spi::master::Spi,
};

use mipidsi::models::ST7789;
use embedded_graphics::{prelude::{DrawTarget, Dimensions}, pixelcolor::Rgb565, Pixel, primitives::Rectangle};

type Dc = GpioPin<Output<PushPull>, 4>;
type Reset = GpioPin<Output<PushPull>, 48>;

pub struct EmbassyTaskDisplay<'a> {
    pub display: mipidsi::Display<SPIInterfaceNoCS<Spi<'a, SPI2, FullDuplexMode>, Dc>, ST7789, Reset>,
}

impl DrawTarget for EmbassyTaskDisplay<'static> {
    type Color = Rgb565;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
    }
}

impl Dimensions for EmbassyTaskDisplay<'static> {
    fn bounding_box(&self) -> Rectangle {
        self.display.bounding_box()
    }
}

impl<'a, 'b> DrawTarget for &'a mut EmbassyTaskDisplay<'b> {
    type Color = Rgb565;
    type Error = DisplayError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.display.draw_iter(pixels)
    }
}

impl<'a, 'b> Dimensions for &'a mut EmbassyTaskDisplay<'b> {
    fn bounding_box(&self) -> Rectangle {
        self.display.bounding_box()
    }
}

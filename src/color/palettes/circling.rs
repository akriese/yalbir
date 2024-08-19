use crate::color::Rgb;

use super::Palette;

struct Circling {
    degrees_per_step: f32,
    clockwise: bool,
}

impl Palette for Circling {
    fn next(&mut self, input: Rgb) -> Rgb {
        let x = colorsys::Rgb::default();
        todo!()
    }

    fn primary(&self) -> Rgb {
        todo!()
    }

    fn secondary(&self) -> Rgb {
        todo!()
    }

    fn tertiary(&self) -> Rgb {
        todo!()
    }
}

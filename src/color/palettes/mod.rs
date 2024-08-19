use super::Rgb;

pub mod circling;

trait Palette {
    /// Calculates the next color value in the palette
    fn next(&mut self, input: Rgb) -> Rgb;

    /// Calculates the next color value in the palette
    // fn next_bulk(&mut self, input: &Vec<Rgb>) -> Rgb;

    /// Returns the primary color value
    fn primary(&self) -> Rgb;

    /// Returns the secondary color value
    fn secondary(&self) -> Rgb;

    /// Returns the tertiary color value
    fn tertiary(&self) -> Rgb;
}

use core::num::ParseIntError;

use esp_hal::rng::Rng;

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn random(rng: &mut Rng, max_intensity: u8) -> Self {
        let mut res = Self::default();
        res.fill_random(rng, max_intensity);
        res
    }

    pub fn fill_random(&mut self, rng: &mut Rng, max_intensity: u8) {
        self.r = rng.random() as u8 % max_intensity;
        self.g = rng.random() as u8 % max_intensity;
        self.b = rng.random() as u8 % max_intensity;
    }

    pub fn scale(&mut self, scale: u8) {
        self.r = ((self.r as u32 * scale as u32) / 100) as u8;
        self.g = ((self.g as u32 * scale as u32) / 100) as u8;
        self.b = ((self.b as u32 * scale as u32) / 100) as u8;
    }

    pub fn scaled(&self, scale: u8) -> Rgb {
        let mut copy = self.clone();
        copy.scale(scale);
        copy
    }

    pub fn add(&mut self, rhs: &Rgb) {
        self.r = self.r.saturating_add(rhs.r);
        self.g = self.g.saturating_add(rhs.g);
        self.b = self.b.saturating_add(rhs.b);
    }

    pub fn from(hex_str: &str) -> Result<Self, ParseIntError> {
        if hex_str.len() != 6 {
            return Ok(Self::default());
        }

        let r = u8::from_str_radix(&hex_str[0..2], 16)?;
        let g = u8::from_str_radix(&hex_str[2..4], 16)?;
        let b = u8::from_str_radix(&hex_str[4..6], 16)?;

        Ok(Self { r, g, b })
    }

    pub fn random_with_variation(base_color: &Self, variation: &Self, rng: &mut Rng) -> Self {
        let r_var = ((rng.random() % 100) as i32 - 50) * variation.r as i32 / 50;
        let g_var = ((rng.random() % 100) as i32 - 50) * variation.g as i32 / 50;
        let b_var = ((rng.random() % 100) as i32 - 50) * variation.b as i32 / 50;

        Self {
            r: (base_color.r as i32 + r_var).max(0).min(255) as u8,
            g: (base_color.g as i32 + g_var).max(0).min(255) as u8,
            b: (base_color.b as i32 + b_var).max(0).min(255) as u8,
        }
    }
}

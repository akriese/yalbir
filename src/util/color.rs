use esp_hal::rng::Rng;

#[derive(Copy, Clone, Default, Debug)]
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
}

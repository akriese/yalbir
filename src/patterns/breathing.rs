use super::LedPattern;
use crate::{util::color::Rgb, N_LEDS, RENDERS_PER_SECOND};
use esp_hal::rng::Rng;

pub struct Breathing {
    rgbs_max: [Rgb; N_LEDS],
    rgbs_current: [Rgb; N_LEDS],
    current_intensity: f32,
    direction_up: bool,
    speed: f32,
}

pub enum BreathingMode {
    Single,
    Double,
    Mixed,
}

impl Breathing {
    /// Creates a new breathing pattern.
    ///
    /// * `mode`: [TODO:parameter]
    /// * `max_intensity`: [TODO:parameter]
    /// * `rng`: [TODO:parameter]
    /// * `speed`: 1.0 -> once per second;
    pub fn new(mode: BreathingMode, max_intensity: u8, rng: &mut Rng, speed: f32) -> Self {
        let mut res = Self {
            rgbs_max: [Rgb::default(); N_LEDS],
            rgbs_current: [Rgb::default(); N_LEDS],
            current_intensity: 0.0,
            direction_up: true,
            speed,
        };

        res.rgbs_max
            .iter_mut()
            .for_each(|col| col.fill_random(rng, max_intensity));

        res.rgbs_current = res.rgbs_max;

        res
    }
}

impl LedPattern for Breathing {
    fn next(&mut self) -> &[Rgb] {
        if self.direction_up {
            self.current_intensity += self.speed;
            if self.current_intensity >= RENDERS_PER_SECOND as f32 / 2.0 {
                self.direction_up = false;
                self.current_intensity = RENDERS_PER_SECOND as f32 / 2.0;
            }
        } else {
            self.current_intensity -= self.speed;
            if self.current_intensity <= 0.0 {
                self.direction_up = true;
                self.current_intensity = 0.0;
            }
        }

        for (max, curr) in self.rgbs_max.iter().zip(self.rgbs_current.iter_mut()) {
            *curr = *max;
            curr.scale(self.current_intensity as u8);
        }

        &self.rgbs_current
    }

    fn beat(&mut self) {}
}

use super::LedPattern;
use crate::{util::color::Rgb, N_LEDS, RENDERS_PER_SECOND};
use esp_hal::rng::Rng;

pub struct Breathing<const C: usize> {
    rgbs_max: [Rgb; C],
    rgbs_current: [Rgb; C],
    current_intensity: f32,
    direction_up: bool,
    speed: f32,
}

pub enum BreathingMode {
    Single,
    Double,
    Mixed,
}

impl<const C: usize> Breathing<C> {
    /// Creates a new breathing pattern.
    ///
    /// * `mode`: [TODO:parameter]
    /// * `max_intensity`: [TODO:parameter]
    /// * `rng`: [TODO:parameter]
    /// * `speed`: 1.0 -> once per second;
    pub fn new(mode: BreathingMode, max_intensity: u8, mut rng: Rng, speed: f32) -> Self {
        let mut res = Self {
            rgbs_max: [Rgb::default(); C],
            rgbs_current: [Rgb::default(); C],
            current_intensity: 0.0,
            direction_up: true,
            speed,
        };

        res.rgbs_max
            .iter_mut()
            .for_each(|col| col.fill_random(&mut rng, max_intensity));

        res.rgbs_current = res.rgbs_max;

        res
    }
}

impl<const C: usize> LedPattern for Breathing<C> {
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

use esp_hal::rng::Rng;

use crate::{util::color::Rgb, N_LEDS};

pub struct Breathing {
    rgbs_max: [Rgb; N_LEDS],
    rgbs_current: [Rgb; N_LEDS],
    current_intensity: i8,
    direction_up: bool,
    speed: u8,
}

pub enum BreathingMode {
    Single,
    Double,
    Mixed,
}

impl Breathing {
    pub fn new(mode: BreathingMode, max_intensity: u8, rng: &mut Rng, speed: u8) -> Self {
        let mut res = Self {
            rgbs_max: [Rgb::default(); N_LEDS],
            rgbs_current: [Rgb::default(); N_LEDS],
            current_intensity: 0,
            direction_up: true,
            speed,
        };

        res.rgbs_max
            .iter_mut()
            .for_each(|col| col.fill_random(rng, max_intensity));

        res.rgbs_current = res.rgbs_max.clone();

        res
    }

    pub fn next(&mut self) -> &[Rgb; N_LEDS] {
        if self.direction_up {
            self.current_intensity += self.speed as i8;
            if self.current_intensity >= 100 {
                self.direction_up = false;
                self.current_intensity = 100;
            }
        } else {
            self.current_intensity -= self.speed as i8;
            if self.current_intensity <= 0 {
                self.direction_up = true;
                self.current_intensity = 0;
            }
        }

        for (max, curr) in self.rgbs_max.iter().zip(self.rgbs_current.iter_mut()) {
            *curr = max.clone();
            curr.scale(self.current_intensity as u8);
        }

        &self.rgbs_current
    }
}

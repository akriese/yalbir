use super::{LedPattern, PatternCommand};
use crate::{beat::BeatCount, util::color::Rgb, RENDERS_PER_SECOND};
use alloc::{vec, vec::Vec};
use esp_hal::rng::Rng;

pub struct Breathing {
    rgbs_max: Vec<Rgb>,
    rgbs_current: Vec<Rgb>,
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
    /// * `n_leds`: [TODO:parameter]
    /// * `mode`: [TODO:parameter]
    /// * `max_intensity`: [TODO:parameter]
    /// * `rng`: [TODO:parameter]
    /// * `speed`: 1.0 -> once per second;
    pub fn new(
        n_leds: usize,
        mode: BreathingMode,
        max_intensity: u8,
        mut rng: Rng,
        speed: f32,
    ) -> Self {
        let mut res = Self {
            rgbs_max: vec![Rgb::default(); n_leds],
            rgbs_current: vec![Rgb::default(); n_leds],
            current_intensity: 0.0,
            direction_up: true,
            speed,
        };

        res.rgbs_max
            .iter_mut()
            .for_each(|col| col.fill_random(&mut rng, max_intensity));

        res.rgbs_current.copy_from_slice(&res.rgbs_max[..]);

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

    fn beat(&mut self, beat_info: &BeatCount) {}

    fn size(&self) -> usize {
        self.rgbs_max.len()
    }

    fn from_str(args: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl PatternCommand for Breathing {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        todo!();
    }
}

use super::{LedPattern, PatternCommand, PatternSpeed};
use crate::{beat::BeatCount, util::color::Rgb, RENDERS_PER_SECOND};
use alloc::{vec, vec::Vec};
use esp_hal::rng::Rng;

pub enum StrobeMode {
    Single,
    Individual,
    Unison,
}

pub struct Strobe {
    rgbs: Vec<Rgb>,
    status: Vec<bool>,
    counters: Vec<usize>,
    speed: usize, // switches per second; if 0, listens to beat
    mode: StrobeMode,
    _rng: Rng,
}

impl Strobe {
    pub fn new(n_leds: usize, mode: StrobeMode, rng: Rng, speed: usize) -> Self {
        let mut ret = Self {
            rgbs: vec![Rgb::default(); n_leds],
            status: vec![false; n_leds],
            counters: vec![0; n_leds],
            speed,
            mode,
            _rng: rng,
        };

        match ret.mode {
            StrobeMode::Single => {
                let first = ret._rng.random() as usize % n_leds;

                ret.status[first] = true;
            }
            StrobeMode::Individual => {
                for el in ret.status.iter_mut() {
                    *el = ret._rng.random() % 2 == 1;
                }

                for el in ret.counters.iter_mut() {
                    *el = ret._rng.random() as usize % RENDERS_PER_SECOND;
                }
            }
            StrobeMode::Unison => {}
        }

        ret
    }

    fn trigger(&mut self) {
        match self.mode {
            StrobeMode::Single => {
                let on_idx = self.status.iter().position(|x| *x).unwrap_or(0);

                let mut new_on_idx = on_idx;

                // make sure to get a different index
                while new_on_idx == on_idx {
                    new_on_idx = self._rng.random() as usize % self.size();
                }

                self.status[on_idx] = false;
                self.counters[on_idx] = 0;
                self.status[new_on_idx] = true;
            }
            StrobeMode::Individual => {
                for (s, c) in self.status.iter_mut().zip(self.counters.iter_mut()) {
                    *c += 1;
                    if *c >= 2 || self._rng.random() % 2 == 0 {
                        *c = 0;
                        *s = !*s;
                    }
                }
            }
            StrobeMode::Unison => {
                let new_status = !self.status[0];
                self.status.iter_mut().for_each(|s| *s = new_status);
                self.counters[0] = 0;
            }
        }
    }
}

impl LedPattern for Strobe {
    fn next(&mut self) -> &[Rgb] {
        if self.speed != 0 {
            match self.mode {
                StrobeMode::Single => {
                    let on_idx = self.status.iter().position(|x| *x).unwrap_or(0);

                    self.counters[on_idx] += self.speed;

                    // switch to different index to turn on
                    if self.counters[on_idx] >= RENDERS_PER_SECOND {
                        self.trigger();
                    }
                }
                StrobeMode::Individual => {
                    for (s, c) in self.status.iter_mut().zip(self.counters.iter_mut()) {
                        *c += self.speed;
                        if *c >= RENDERS_PER_SECOND {
                            *c = self._rng.random() as usize % RENDERS_PER_SECOND;
                            *s = !*s;
                        }
                    }
                }
                StrobeMode::Unison => {
                    self.counters[0] += self.speed;
                    if self.counters[0] >= RENDERS_PER_SECOND {
                        self.trigger();
                    }
                }
            }
        }

        for (status, rgb) in self.status.iter().zip(self.rgbs.iter_mut()) {
            if *status {
                *rgb = Rgb {
                    r: 50,
                    g: 50,
                    b: 50,
                };
            } else {
                *rgb = Rgb::default();
            }
        }

        &self.rgbs
    }

    fn beat(&mut self, beat_info: &BeatCount) {
        // only react to beat if speed is zero
        if self.speed != 0 {
            return;
        }

        if !PatternSpeed::N16.is_triggered(beat_info) {
            return;
        }

        self.trigger();
    }

    fn size(&self) -> usize {
        self.rgbs.len()
    }
}

impl PatternCommand for Strobe {
    fn execute_command(&mut self, command: &str) -> Result<(), ()> {
        Err(())
    }
}

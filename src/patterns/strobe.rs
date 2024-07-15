use super::{LedPattern, PatternSpeed};
use crate::{beat::BeatCount, util::color::Rgb, RENDERS_PER_SECOND};
use esp_hal::rng::Rng;

pub enum StrobeMode {
    Single,
    Individual,
    Unison,
}

pub struct Strobe<const N: usize> {
    rgbs: [Rgb; N],
    status: [bool; N],
    counters: [usize; N],
    speed: usize, // switches per second; if 0, listens to beat
    mode: StrobeMode,
    _rng: Rng,
}

impl<const N: usize> Strobe<N> {
    pub fn new(mode: StrobeMode, rng: Rng, speed: usize) -> Self {
        let mut ret = Self {
            rgbs: [Rgb::default(); N],
            status: [false; N],
            counters: [0; N],
            speed,
            mode,
            _rng: rng,
        };

        match ret.mode {
            StrobeMode::Single => {
                let first = ret._rng.random() as usize % N;

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
                    new_on_idx = self._rng.random() as usize % N;
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

impl<const N: usize> LedPattern for Strobe<N> {
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
}

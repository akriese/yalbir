use esp_hal::rng::Rng;

use crate::{beat::BeatCount, util::color::Rgb, MAX_INTENSITY};

use super::{LedPattern, PatternCommand, PatternSpeed};

const MAX_SPEED: usize = 1000;

pub struct ShootingStar<const N: usize, const S: usize> {
    rgbs_current: [Rgb; N],
    speed: usize,
    stars: [Star; S],
    shoot_interval: PatternSpeed,
    _step_counter: usize,
    _rng: Rng,
    _max_intensity: usize,
    _tail_length: usize,
    _star_steps_per_move: usize,
}

#[derive(Default, Debug, Copy, Clone)]
struct Star {
    color: Rgb,
    position: usize,
    speed: usize,
    tail_length: usize,
}

impl<const N: usize, const S: usize> ShootingStar<N, S> {
    pub fn new(speed: usize, rng: Rng) -> Self {
        ShootingStar {
            rgbs_current: [Rgb::default(); N],
            speed,
            shoot_interval: PatternSpeed::default(),
            stars: [Star::default(); S],
            _step_counter: 0,
            _rng: rng,
            _max_intensity: MAX_INTENSITY as usize,
            _tail_length: 10,
            _star_steps_per_move: 2,
        }
    }

    pub fn shoot(&mut self, color: Rgb, speed: usize, tail_length: usize) {
        // choose free position in self.stars array
        let index = self
            .stars
            .iter()
            .position(|s| s.speed == 0)
            .unwrap_or_else(|| {
                // if no star is free, destroy the furthest one and use its space
                (0..self.stars.len())
                    .max_by_key(|i| self.stars[*i].position)
                    .unwrap()
            });

        self.stars[index] = Star {
            color,
            position: 0,
            speed,
            tail_length,
        };
    }
}

impl<const N: usize, const S: usize> LedPattern for ShootingStar<N, S> {
    fn next(&mut self) -> &[Rgb] {
        self._step_counter += 1;

        let should_move = self._step_counter * self.speed >= MAX_SPEED;
        if !should_move {
            return &self.rgbs_current;
        }

        for col in self.rgbs_current.iter_mut() {
            *col = Rgb { r: 0, g: 0, b: 0 };
        }

        for s in self.stars.iter_mut() {
            if s.speed == 0 {
                continue;
            }

            s.position += s.speed;

            // deactivate star if it is out of bounds (including the tail)
            if s.position as i32 - s.tail_length as i32 >= N as i32 {
                s.speed = 0;
            }

            for i in 0..s.tail_length {
                let pos = s.position as i32 - i as i32;
                if pos < N as i32 && pos >= 0 {
                    self.rgbs_current[pos as usize]
                        .add(&s.color.scaled(100 - (100 * i / s.tail_length) as u8));
                }
            }
        }

        self._step_counter = 0;
        &self.rgbs_current
    }

    fn beat(&mut self, beat_info: &BeatCount) {
        if !self.shoot_interval.is_triggered(beat_info) {
            return;
        }

        let color = Rgb::random(&mut self._rng, self._max_intensity as u8);

        self.shoot(color, self._star_steps_per_move, self._tail_length)
    }
}

impl<const N: usize, const C: usize> PatternCommand for ShootingStar<N, C> {
    fn execute_command(&mut self, command: &str) -> Result<(), ()> {
        let cmds = command.split(',');

        log::info!("{}", command);

        for cmd in cmds {
            // 'b' => set beat reaction
            // 's' => star speed
            // 'I' => set max intensity
            // 'l' => set tail length

            let set_cmd = cmd.as_bytes()[0] as char;

            match set_cmd {
                'b' => match cmd.as_bytes()[1] as char {
                    '0' => self.shoot_interval = PatternSpeed::N1,
                    '1' => self.shoot_interval = PatternSpeed::N2,
                    '2' => self.shoot_interval = PatternSpeed::N4,
                    '3' => self.shoot_interval = PatternSpeed::N8,
                    '4' => self.shoot_interval = PatternSpeed::N16,
                    '5' => self.shoot_interval = PatternSpeed::N32,
                    'f' => self.shoot_interval.faster(),
                    's' => self.shoot_interval.slower(),
                    _ => return Result::Err(()),
                },
                's' => {
                    let speed = cmd[1..].parse::<usize>().unwrap();
                    self.speed = speed;
                }
                'S' => {
                    let star_speed = cmd[1..].parse::<usize>().unwrap();
                    self._star_steps_per_move = star_speed;

                    for s in self.stars.iter_mut() {
                        s.speed = star_speed;
                    }
                }
                'I' => {
                    let intensity = cmd[1..].parse::<usize>().unwrap();
                    self._max_intensity = intensity;
                }
                'l' => {
                    let length = cmd[1..].parse::<usize>().unwrap();
                    self._tail_length = length;

                    for s in self.stars.iter_mut() {
                        s.tail_length = length;
                    }
                }
                _ => return Result::Err(()),
            };
        }

        Ok(())
    }
}

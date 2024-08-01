use alloc::{vec, vec::Vec};
use anyhow::{anyhow, Error};
use esp_hal::rng::Rng;

use crate::{
    beat::BeatCount,
    patterns::command,
    util::{color::Rgb, random::get_rng},
    MAX_INTENSITY,
};

use super::{LedPattern, PatternCommand, PatternSpeed};

const MAX_SPEED: usize = 1000;

pub struct ShootingStar {
    rgbs_current: Vec<Rgb>,
    speed: usize,
    stars: Vec<Star>,
    shoot_interval: PatternSpeed,
    step_counter: usize,
    rng: Rng,
    max_intensity: usize,
    tail_length: usize,
    star_steps_per_move: usize,
}

#[derive(Default, Debug, Copy, Clone)]
struct Star {
    color: Rgb,
    position: usize,
    speed: usize,
    tail_length: usize,
}

impl ShootingStar {
    pub fn new(n_leds: usize, speed: usize, rng: Rng) -> Self {
        ShootingStar {
            rgbs_current: vec![Rgb::default(); n_leds],
            speed,
            shoot_interval: PatternSpeed::default(),
            stars: vec![],
            step_counter: 0,
            rng,
            max_intensity: MAX_INTENSITY as usize,
            tail_length: 10,
            star_steps_per_move: 2,
        }
    }

    pub fn shoot(&mut self, color: Rgb, speed: usize, tail_length: usize) {
        // choose free position in self.stars
        let index = self.stars.iter().position(|s| s.speed == 0);

        let s = Star {
            color,
            position: 0,
            speed,
            tail_length,
        };

        if let Some(idx) = index {
            self.stars[idx] = s;
        } else {
            // push, if all existing stars are still in bounds
            self.stars.push(s);
        }
    }
}

impl LedPattern for ShootingStar {
    fn next(&mut self) -> &[Rgb] {
        self.step_counter += 1;

        let should_move = self.step_counter * self.speed >= MAX_SPEED;
        if !should_move {
            return &self.rgbs_current;
        }

        for col in self.rgbs_current.iter_mut() {
            *col = Rgb { r: 0, g: 0, b: 0 };
        }

        let size = self.size();
        for s in self.stars.iter_mut() {
            if s.speed == 0 {
                continue;
            }

            s.position += s.speed;

            // deactivate star if it is out of bounds (including the tail)
            if s.position as i32 - s.tail_length as i32 >= size as i32 {
                s.speed = 0;
            }

            for i in 0..s.tail_length {
                let pos = s.position as i32 - i as i32;
                if pos < size as i32 && pos >= 0 {
                    self.rgbs_current[pos as usize]
                        .add(&s.color.scaled(100 - (100 * i / s.tail_length) as u8));
                }
            }
        }

        self.step_counter = 0;
        &self.rgbs_current
    }

    fn size(&self) -> usize {
        self.rgbs_current.len()
    }

    fn beat(&mut self, beat_info: &BeatCount) {
        if !self.shoot_interval.is_triggered(beat_info) {
            return;
        }

        let color = Rgb::random(&mut self.rng, self.max_intensity as u8);

        self.shoot(color, self.star_steps_per_move, self.tail_length)
    }

    fn from_str(command: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let mut args = command.split(',');
        let n_leds = args
            .next()
            .ok_or("No N LED arg given!")
            .map_err(Error::msg)?
            .parse()
            .map_err(Error::msg)?;
        let speed = args
            .next()
            .ok_or("No speed arg given!")
            .map_err(Error::msg)?
            .parse()
            .map_err(Error::msg)?;
        let rng = get_rng();

        Ok(Self::new(n_leds, speed, rng))
    }
}

impl PatternCommand for ShootingStar {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        let cmds = command.split(',');

        log::info!("{}", command);

        for cmd in cmds {
            // 'b' => set beat reaction
            // 's' => star speed
            // 'I' => set max intensity
            // 'l' => set tail length

            let set_cmd = cmd.as_bytes()[0] as char;

            match set_cmd {
                'b' => self.shoot_interval.change(cmd.as_bytes()[1] as char)?,
                's' => {
                    let speed = command::parse(&cmd[1..])?;
                    self.speed = speed;
                }
                'S' => {
                    let star_speed = command::parse(&cmd[1..])?;
                    self.star_steps_per_move = star_speed;

                    for s in self.stars.iter_mut() {
                        s.speed = star_speed;
                    }
                }
                'I' => {
                    let intensity = command::parse(&cmd[1..])?;
                    self.max_intensity = intensity;
                }
                'l' => {
                    let length = command::parse(&cmd[1..])?;
                    self.tail_length = length;

                    for s in self.stars.iter_mut() {
                        s.tail_length = length;
                    }
                }
                c => return Err(anyhow!("Invalid command {} for ShootingStar", c)),
            };
        }

        Ok(())
    }
}

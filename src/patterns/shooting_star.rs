use crate::{util::color::Rgb, N_LEDS};

const MAX_STARS: usize = 20;
const MAX_SPEED: usize = 1000;

pub struct ShootingStar {
    rgbs_current: [Rgb; N_LEDS],
    speed: usize,
    stars: [Star; MAX_STARS],
    _step_counter: usize,
}

#[derive(Default, Debug, Copy, Clone)]
struct Star {
    color: Rgb,
    position: usize,
    speed: usize,
    tail_length: usize,
}

impl ShootingStar {
    pub fn new(speed: usize) -> Self {
        ShootingStar {
            rgbs_current: [Rgb::default(); N_LEDS],
            speed,
            stars: [Star::default(); MAX_STARS],
            _step_counter: 0,
        }
    }

    pub fn next(&mut self) -> &[Rgb; N_LEDS] {
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
            if s.position as i32 - s.tail_length as i32 >= N_LEDS as i32 {
                s.speed = 0;
            }

            for i in 0..s.tail_length {
                let pos = s.position as i32 - i as i32;
                if pos < N_LEDS as i32 && pos >= 0 {
                    self.rgbs_current[pos as usize]
                        .add(&s.color.scaled(100 - (100 * i / s.tail_length) as u8));
                }
            }
        }

        self._step_counter = 0;
        &self.rgbs_current
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

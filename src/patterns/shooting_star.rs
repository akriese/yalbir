use crate::{util::color::Rgb, N_LEDS};

const MAX_STARS: usize = 10;

pub struct ShootingStar {
    rgbs_current: [Rgb; N_LEDS],
    speed_bounds: (u8, u8),
    stars: [Star; MAX_STARS],
}

#[derive(Default, Debug, Copy, Clone)]
struct Star {
    color: Rgb,
    position: usize,
    speed: usize,
    tail_length: usize,
}

impl ShootingStar {
    pub fn new(speed_bounds: (u8, u8)) -> Self {
        ShootingStar {
            rgbs_current: [Rgb::default(); N_LEDS],
            speed_bounds,
            stars: [Star::default(); MAX_STARS],
        }
    }

    pub fn next(&mut self) -> &[Rgb; N_LEDS] {
        for col in self.rgbs_current.iter_mut() {
            *col = Rgb { r: 0, g: 0, b: 0 };
        }

        for s in self.stars.iter_mut() {
            s.position += s.speed as usize;

            for i in 0..s.tail_length {
                let pos = (s.position as i32 - i as i32);
                if pos < N_LEDS as i32 && pos >= 0 {
                    self.rgbs_current[pos as usize]
                        .add(&s.color.scaled(100 - (100 * i / s.tail_length) as u8));
                }
            }
        }

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

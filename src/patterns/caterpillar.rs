use alloc::{vec, vec::Vec};

use crate::{beat::BeatCount, util::color::Rgb};

use super::{LedPattern, PatternCommand, PatternSpeed};

// a single caterpillar
struct CaterPillar {
    pos: (usize, usize),     // first position, last position
    goal: usize,             // end position for the current move
    lengths: (usize, usize), // shortest length, longest length
    speeds: (usize, usize),  // move distance per step, (while head/tail move)
    waiting_time: usize,     // steps to wait before starting a new move
    head_color: Rgb,         // color of the head
    body_color: Rgb,         // color of the rest of the the body
    head_moving: bool,       // true: head moving; false: tail moving
    wait_counter: usize,     // counter for how many steps the caterpillar hasnt moved
}

pub struct CaterPillars {
    rgbs: Vec<Rgb>,
    caterpillars: Vec<CaterPillar>,
    beat_reaction: Option<PatternSpeed>, // all caterpillars finish their current move
    needs_to_finish: bool,               // indicator that tells next() to finish a move
    step_counter: usize,                 // internal next() step counter
    spawn_rate: usize,                   // every n next() spawns a new caterpillar
}

impl CaterPillar {
    fn finish_current_move(&mut self) {
        if self.head_moving {
            self.pos.0 = self.goal;
        } else {
            self.pos.1 = self.goal;
        }
    }

    fn init_next_move(&mut self) {
        self.head_moving = !self.head_moving;

        self.goal = if self.head_moving {
            self.pos.1 + self.lengths.1
        } else {
            self.pos.0 - self.lengths.0
        }
    }

    // returns true if the current move is finished
    fn maybe_move(&mut self, counter: usize) -> bool {
        let current_speed = if self.head_moving {
            self.speeds.0
        } else {
            self.speeds.1
        };

        if counter % current_speed == 0 {
            if self.head_moving {
                self.pos.0 += 1;
                return self.pos.0 == self.goal;
            } else {
                self.pos.1 += 1;
                return self.pos.1 == self.goal;
            }
        }

        false
    }
}

impl CaterPillars {
    pub fn new(n_leds: usize, beat_reaction: Option<PatternSpeed>, spawn_rate: usize) -> Self {
        Self {
            rgbs: vec![Rgb::default(); n_leds],
            caterpillars: vec![],
            beat_reaction,
            needs_to_finish: false,
            step_counter: 0,
            spawn_rate,
        }
    }

    fn add_new_caterpillar(&mut self) {
        let mut new_cp = CaterPillar {
            pos: (0, 0),
            goal: 0,
            lengths: (5, 15),
            speeds: (2, 3),
            waiting_time: 10,
            head_color: Rgb {
                r: 80,
                g: 10,
                b: 10,
            },
            body_color: Rgb {
                r: 10,
                g: 80,
                b: 10,
            },
            head_moving: false,
            wait_counter: 0,
        };
        new_cp.init_next_move();
        let search_out_of_bounds = self
            .caterpillars
            .iter()
            .position(|cp| cp.pos.1 >= self.size());

        // recycle caterpillars that went out of scope
        if let Some(idx) = search_out_of_bounds {
            self.caterpillars[idx] = new_cp;
        } else {
            self.caterpillars.push(new_cp);
        }
    }
}

impl LedPattern for CaterPillars {
    fn next(&mut self) -> &[Rgb] {
        self.step_counter += 1;
        let n_leds = self.size();

        if self.step_counter % self.spawn_rate == 0 {
            self.add_new_caterpillar();
        }

        // move every caterpillar, either head or tail
        // but only according to their speeds:
        // the individual speeds (head and tail) tell, how often the caterpillar moves
        // on next() calls (using modulo). Higher speed here means slower movement, so
        // this naming is not good yet xD
        if self.beat_reaction.is_none() {
            for cp in self.caterpillars.iter_mut() {
                if cp.pos.1 >= n_leds {
                    continue;
                }

                // check if cp is waiting right now
                if cp.wait_counter != 0 {
                    cp.wait_counter += 1;
                    if cp.wait_counter == cp.waiting_time {
                        cp.init_next_move();
                        cp.wait_counter = 0;
                    }

                    continue;
                }

                let move_finished = cp.maybe_move(self.step_counter);
                if move_finished {
                    cp.wait_counter = 1;
                }
            }
        }

        // if instead, beat_reaction is used, the individual speeds are disregarded
        // and instead the needed movement per step is calculated for each caterpillar
        // to end the each movement on a beat, for that, the `needs_to_finish` bool is
        // used as a signal to finish the current movement
        if self.beat_reaction.is_some() {
            if self.needs_to_finish {
                self.needs_to_finish = false;

                for cp in self.caterpillars.iter_mut() {
                    cp.finish_current_move();
                    cp.init_next_move();
                }
            } else {
            }
        }

        // reset the rgb vector
        self.rgbs.iter_mut().for_each(|rgb| *rgb = Rgb::default());

        // fill the rgb vector
        for cp in self.caterpillars.iter() {
            for pos in cp.pos.1..(cp.pos.0).min(self.size() - 1) {
                self.rgbs[pos] = cp.body_color;
            }

            if cp.pos.0 < self.size() {
                self.rgbs[cp.pos.0] = cp.head_color;
            }
        }

        &self.rgbs
    }

    fn beat(&mut self, beat_info: &BeatCount) {
        if let Some(br) = self.beat_reaction {
            if br.is_triggered(beat_info) {
                self.needs_to_finish = true;
            }
        }
    }

    fn size(&self) -> usize {
        self.rgbs.len()
    }
}

impl PatternCommand for CaterPillars {
    fn execute_command(&mut self, command: &str) -> Result<(), ()> {
        todo!();
    }
}

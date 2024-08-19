use alloc::{vec, vec::Vec};
use esp_hal::rng::Rng;
use nom::{
    bytes::complete::tag,
    character::complete::{alpha0, u32},
    sequence::tuple,
};

use crate::{
    beat::BeatCount,
    color::Rgb,
    patterns::{command, invalid_cmd},
    util::random::get_rng,
};

use super::{LedPattern, PatternCommand, PatternSpeed};

use anyhow::anyhow;

// a single caterpillar
#[derive(Debug, Copy, Clone)]
struct CaterPillar {
    pos: (usize, usize),         // first position, last position
    goal: usize,                 // end position for the current move
    lengths: (usize, usize),     // shortest length, longest length
    speeds: (usize, usize),      // move distance per step, (while head/tail move)
    waiting_time: usize,         // steps to wait before starting a new move
    head_color: Rgb,             // color of the head
    body_color: Rgb,             // color of the rest of the the body
    head_moving: bool,           // true: head moving; false: tail moving
    wait_counter: Option<usize>, // counter for how many steps the caterpillar hasnt moved
}

struct CreationParams {
    lengths: (usize, usize),
    speeds: (usize, usize),
    waiting_time: usize,
    head_color: Rgb,
    head_color_variation: Rgb,
    body_color: Rgb,
    body_color_variation: Rgb,
}

pub struct CaterPillars {
    rgbs: Vec<Rgb>,
    caterpillars: Vec<CaterPillar>,
    beat_reaction: Option<PatternSpeed>, // all caterpillars finish their current move
    needs_to_finish: bool,               // indicator that tells next() to finish a move
    step_counter: usize,                 // internal next() step counter
    spawn_rate: usize,                   // every n next() spawns a new caterpillar
    new_pillar_params: CreationParams,
    rng: Rng,
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
    pub fn new(
        n_leds: usize,
        beat_reaction: Option<PatternSpeed>,
        spawn_rate: usize,
        rng: Rng,
    ) -> Self {
        Self {
            rgbs: vec![Rgb::default(); n_leds],
            caterpillars: vec![],
            beat_reaction,
            needs_to_finish: false,
            step_counter: 0,
            spawn_rate,
            new_pillar_params: CreationParams {
                lengths: (5, 15),
                speeds: (2, 8),
                waiting_time: 2,
                head_color: Rgb::from("401010").unwrap(),
                head_color_variation: Rgb::from("301010").unwrap(),
                body_color: Rgb::from("105010").unwrap(),
                body_color_variation: Rgb::from("102010").unwrap(),
            },
            rng,
        }
    }

    fn add_new_caterpillar(&mut self) {
        let p = &self.new_pillar_params;

        let mut new_cp = CaterPillar {
            pos: (0, 0),
            goal: 0,
            lengths: p.lengths,
            speeds: p.speeds,
            waiting_time: p.waiting_time,
            head_color: Rgb::random_with_variation(
                &p.head_color,
                &p.head_color_variation,
                &mut self.rng,
            ),
            body_color: Rgb::random_with_variation(
                &p.body_color,
                &p.body_color_variation,
                &mut self.rng,
            ),
            head_moving: false,
            wait_counter: None,
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
                if let Some(c) = cp.wait_counter.as_mut() {
                    *c += 1;
                    if *c >= cp.waiting_time {
                        cp.wait_counter = None;
                        cp.init_next_move();
                    }

                    continue;
                }

                let move_finished = cp.maybe_move(self.step_counter);
                if move_finished {
                    cp.wait_counter = Some(0);
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

    fn from_str(args: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (_remainder, (n_leds, _, reaction, _, speed)) =
            tuple((u32, tag(","), alpha0, tag(","), u32))(args).map_err(
                |err: nom::Err<nom::error::Error<&str>>| {
                    anyhow!(
                        "Problem while parsing args for Caterpillars: {:?}; {:?}",
                        args,
                        err
                    )
                },
            )?;
        let beat_reaction = if reaction.is_empty() {
            None
        } else {
            Some(PatternSpeed::try_from(reaction.chars().next().unwrap())?)
        };
        let rng = get_rng();

        Ok(Self::new(
            n_leds as usize,
            beat_reaction,
            speed as usize,
            rng,
        ))
    }
}

impl PatternCommand for CaterPillars {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        // set: spawn_rate, beat reaction
        // set for random generation: length range, speed range, waiting time,
        // colors
        let cmds = command.split(',');

        log::info!("{}", command);

        for cmd in cmds {
            let set_cmd = cmd.as_bytes()[0] as char;

            match set_cmd {
                'b' => {
                    if self.beat_reaction.is_none() {
                        self.beat_reaction = Some(PatternSpeed::default());
                    }
                    self.beat_reaction
                        .unwrap()
                        .change(cmd.as_bytes()[1] as char)?;
                }
                's' => {
                    let spawn_rate = cmd[1..].parse::<usize>().unwrap();
                    self.spawn_rate = spawn_rate;
                }
                'L' => {
                    self.new_pillar_params.lengths = command::parse_tuple(&cmd[1..])?;
                }
                'S' => {
                    self.new_pillar_params.speeds = command::parse_tuple(&cmd[1..])?;
                }
                'W' => {
                    self.new_pillar_params.waiting_time = command::parse(&cmd[1..])?;
                }
                'H' => {
                    self.new_pillar_params.head_color = command::parse_rgb(&cmd[1..])?;
                }
                'h' => {
                    self.new_pillar_params.head_color_variation = command::parse_rgb(&cmd[1..])?;
                }
                'T' => {
                    self.new_pillar_params.body_color = command::parse_rgb(&cmd[1..])?;
                }
                't' => {
                    self.new_pillar_params.body_color_variation = command::parse_rgb(&cmd[1..])?;
                }
                c => return Err(anyhow!("Invalid command {} for Caterpillars", c)),
            };
        }

        Ok(())
    }
}

use super::{LedPattern, PatternCommand, PatternSpeed};
use crate::{
    beat::BeatCount,
    color::Rgb,
    patterns::invalid_cmd,
    util::random::get_rng,
    RENDERS_PER_SECOND,
};
use alloc::{vec, vec::Vec};
use anyhow::anyhow;
use esp_hal::rng::Rng;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, u32},
    combinator::value,
    sequence::tuple,
    IResult,
};

#[derive(Clone)]
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
    rng: Rng,
    max_intensity: u8,
    beat_reaction: PatternSpeed,
}

// how many next() calls the leds stay turned on for a strobe
const MAX_STROBE_ON_DURATION: usize = 3;

impl Strobe {
    pub fn new(n_leds: usize, mode: StrobeMode, rng: Rng, speed: usize) -> Self {
        let mut ret = Self {
            rgbs: vec![Rgb::default(); n_leds],
            status: vec![false; n_leds],
            counters: vec![0; n_leds],
            speed,
            mode,
            rng,
            max_intensity: 50,
            beat_reaction: PatternSpeed::default(),
        };

        match ret.mode {
            StrobeMode::Single => {
                let first = ret.rng.random() as usize % n_leds;

                ret.status[first] = true;
            }
            StrobeMode::Individual => {
                for el in ret.status.iter_mut() {
                    *el = ret.rng.random() % 2 == 1;
                }

                for el in ret.counters.iter_mut() {
                    *el = ret.rng.random() as usize % RENDERS_PER_SECOND;
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
                    new_on_idx = self.rng.random() as usize % self.size();
                }

                self.status[on_idx] = false;
                self.counters[on_idx] = 0;
                self.status[new_on_idx] = true;
            }
            StrobeMode::Individual => {
                for (s, c) in self.status.iter_mut().zip(self.counters.iter_mut()) {
                    *c += 1;
                    if *c >= 2 || self.rng.random() % 2 == 0 {
                        *c = 0;
                        *s = !*s;
                    }
                }
            }
            StrobeMode::Unison => {
                // in Unison mode, trigger only activates the LEDs
                // the deactivation happens in the next() function
                self.status.iter_mut().for_each(|s| *s = true);
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
                            *c = self.rng.random() as usize % RENDERS_PER_SECOND;
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

        // increase strobe on counter
        if let StrobeMode::Unison = self.mode {
            if self.status[0] {
                self.counters[1] += 1;

                if self.counters[1] > MAX_STROBE_ON_DURATION {
                    self.counters[1] = 0;
                    self.status.iter_mut().for_each(|s| *s = false);
                }
            }
        }

        for (status, rgb) in self.status.iter().zip(self.rgbs.iter_mut()) {
            if *status {
                *rgb = Rgb {
                    r: self.max_intensity,
                    g: self.max_intensity,
                    b: self.max_intensity,
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

        if !self.beat_reaction.is_triggered(beat_info) {
            return;
        }

        self.trigger();
    }

    fn size(&self) -> usize {
        self.rgbs.len()
    }

    fn from_str(command: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (_remainder, (n_leds, _, mode, _, speed)) =
            tuple((u32, tag(","), alpha1, tag(","), u32))(command).map_err(
                |err: nom::Err<nom::error::Error<&str>>| {
                    anyhow!(
                        "Problem while parsing args for ShootingStar: {:?}; {:?}",
                        command,
                        err
                    )
                },
            )?;
        let (_, mode) =
            parse_mode(mode).map_err(|_err| anyhow!("Invalid StrobeMode {:?}", mode))?;
        let rng = get_rng();

        Ok(Self::new(n_leds as usize, mode, rng, speed as usize))
    }
}

fn parse_mode(input: &str) -> IResult<&str, StrobeMode> {
    alt((
        value(StrobeMode::Single, tag("s")),
        value(StrobeMode::Individual, tag("i")),
        value(StrobeMode::Unison, tag("u")),
    ))(input)
}

static COMMAND_HELP: &str =
    "b<char> - Beat reaction; s<int> - speed; I<u8> - intensity; m[s,i,u] - mode switch";

impl PatternCommand for Strobe {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        let cmds = command.split(',');

        log::info!("{}", command);

        for cmd in cmds {
            // 'b' => set beat reaction
            // 's' => flicker speed (discoupling from beat reaction)
            // 'I' => set max intensity

            let set_cmd = cmd.as_bytes()[0] as char;

            match set_cmd {
                'b' => {
                    let arg = &cmd.as_bytes()[1..];
                    if arg.len() != 1 {
                        return Err(anyhow!("Beat reaction arg must be exactly one char!"));
                    }
                    self.beat_reaction.change(arg[0] as char)?;

                    // reset speed as only then, the beat reaction is used
                    self.speed = 0;
                }
                's' => {
                    let speed = cmd[1..].parse::<usize>().map_err(|e| {
                        anyhow!("Speed arg {:?} could not parsed! {:?}", &cmd[1..], e)
                    })?;
                    self.speed = speed;
                }
                'I' => {
                    let intensity = cmd[1..].parse::<u8>().map_err(|e| {
                        anyhow!(
                            "The intensity arg {:?} could not be parsed! {:?}",
                            &cmd[1..],
                            e
                        )
                    })?;
                    self.max_intensity = intensity;
                }
                _ => return invalid_cmd("Strobe", cmd, COMMAND_HELP),
            };
        }

        Ok(())
    }
}

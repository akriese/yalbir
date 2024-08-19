use super::{LedPattern, PatternCommand};
use crate::{
    beat::BeatCount, color::Rgb, patterns::invalid_cmd, util::random::get_rng, RENDERS_PER_SECOND,
};
use alloc::{vec, vec::Vec};
use anyhow::anyhow;
use esp_hal::rng::Rng;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha0, u32, u8},
    combinator::value,
    number::complete::float,
    sequence::tuple,
    IResult,
};

pub struct Breathing {
    rgbs_max: Vec<Rgb>,
    rgbs_current: Vec<Rgb>,
    current_intensity: f32,
    direction_up: bool,
    speed: f32,
    max_intensity: u8,
    mode: BreathingMode,
    rng: Rng,
}

#[derive(Clone)]
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
        rng: Rng,
        speed: f32,
    ) -> Self {
        let mut res = Self {
            rgbs_max: vec![Rgb::default(); n_leds],
            rgbs_current: vec![Rgb::default(); n_leds],
            current_intensity: 0.0,
            direction_up: true,
            speed,
            max_intensity,
            mode,
            rng,
        };

        res.switch_colors();

        res
    }
}

impl Breathing {
    fn switch_colors(&mut self) {
        self.rgbs_max
            .iter_mut()
            .for_each(|col| col.fill_random(&mut self.rng, self.max_intensity));

        self.rgbs_current.copy_from_slice(&self.rgbs_max[..]);
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

                if let BreathingMode::Mixed = self.mode {
                    self.switch_colors();
                }
            }
        }

        for (max, curr) in self.rgbs_max.iter().zip(self.rgbs_current.iter_mut()) {
            *curr = *max;
            curr.scale(self.current_intensity as u8);
        }

        &self.rgbs_current
    }

    fn beat(&mut self, _beat_info: &BeatCount) {}

    fn size(&self) -> usize {
        self.rgbs_max.len()
    }

    fn from_str(args: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let (_remainder, (n_leds, _, mode, _, intensity, _, speed)) =
            tuple((u32, tag(","), alpha0, tag(","), u8, tag(","), float))(args).map_err(
                |err: nom::Err<nom::error::Error<&str>>| {
                    anyhow!(
                        "Problem while parsing args for Caterpillars: {:?}; {:?}",
                        args,
                        err
                    )
                },
            )?;
        let (_, mode) =
            parse_mode(mode).map_err(|_err| anyhow!("Invalid Caterpillars mode {:?}", mode))?;
        let rng = get_rng();

        Ok(Self::new(n_leds as usize, mode, intensity, rng, speed))
    }
}

fn parse_mode(input: &str) -> IResult<&str, BreathingMode> {
    alt((
        value(BreathingMode::Single, tag("s")),
        value(BreathingMode::Double, tag("d")),
        value(BreathingMode::Mixed, tag("m")),
    ))(input)
}

static COMMAND_HELP: &str = "s<float> - speed (frequency 1/s); I<u8> - max intensity";

impl PatternCommand for Breathing {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        let cmds = command.split(',');

        for cmd in cmds {
            let set_cmd = cmd.as_bytes()[0] as char;

            match set_cmd {
                's' => {
                    let speed = cmd[1..].parse::<f32>().map_err(|e| {
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
                'm' => {
                    let arg = &cmd[1..];
                    if arg.len() != 1 {
                        return Err(anyhow!("Mode arg must be exactly one char!"));
                    }

                    let (_, mode) = parse_mode(arg)
                        .map_err(|_| anyhow!("Invalid BreathingMode. Use [s,d,m]!"))?;
                    self.mode = mode;
                }
                _ => return invalid_cmd("Breathing", cmd, COMMAND_HELP),
            };
        }

        Ok(())
    }
}

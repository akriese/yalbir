use alloc::{boxed::Box, vec, vec::Vec};
use anyhow::{anyhow, Error};
use nom::bytes::complete::take_while;

use crate::{beat::BeatCount, color::Rgb};
use core::str;

use super::{
    command::range_tuple, pattern_with_args_from_command, LedPattern, PatternCommand, PatternKind,
};

struct PatternSection {
    range: (usize, usize),
    pattern: Box<dyn LedPattern>,
}

// PatternSection, rendering status, beat listening status
type PatternWithStatus = (PatternSection, bool, bool);

pub struct PartitionedPatterns {
    rgbs: Vec<Rgb>,
    patterns: Vec<PatternWithStatus>,
}

impl PartitionedPatterns {
    pub fn new(n_leds: usize) -> Self {
        Self {
            rgbs: vec![Rgb::default(); n_leds],
            patterns: vec![],
        }
    }

    pub fn add(&mut self, pattern: Box<dyn LedPattern>, range: Option<(usize, usize)>) {
        // if no range given, get it from the last added pattern and the given pattern's size
        // this is obviously not very robust if the user adds new patterns in not-sorted order
        // So, there needs to happen some adaptation later...
        let _range = range.or_else(|| {
            if self.patterns.is_empty() {
                Some((0, pattern.size()))
            } else {
                let last_end = self.patterns.last().unwrap().0.range.1;
                Some((last_end, last_end + pattern.size()))
            }
        });

        self.patterns.push((
            PatternSection {
                pattern,
                range: _range.unwrap(),
            },
            true,
            true,
        ));
    }
}

impl LedPattern for PartitionedPatterns {
    fn next(&mut self) -> &[Rgb] {
        self.rgbs.iter_mut().for_each(|rgb| *rgb = Rgb::default());

        for (ps, render_status, _beat_status) in self.patterns.iter_mut() {
            if *render_status {
                let rgbs = ps.pattern.as_mut().next();
                let (a, b) = (ps.range.0, ps.range.1);
                self.rgbs[a..b].copy_from_slice(&rgbs[..b - a]);
            }
        }
        &self.rgbs
    }

    fn beat(&mut self, beat_info: &BeatCount) {
        for (ps, _render_status, beat_status) in self.patterns.iter_mut() {
            if *beat_status {
                ps.pattern.beat(beat_info);
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
        let mut args = args.split(',');
        let size = args
            .next()
            .expect("No size arg given!")
            .parse::<usize>()
            .map_err(Error::msg)?;

        Ok(Self {
            rgbs: vec![Rgb::default(); size],
            patterns: vec![],
        })
    }
}

impl PatternCommand for PartitionedPatterns {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        let cmds = command.split(';');

        for cmd in cmds {
            let cmd_bytes = cmd.as_bytes();

            // "pn..." => regards pattern n
            // "g..." => global execution (stop all, resume all, etc.)
            // "a..." => add new pattern

            match cmd_bytes[0] as char {
                'p' => {
                    // parse the pattern index, this assumes that the max index is 9
                    let index = (cmd_bytes[1] - b'0') as usize;
                    if index > self.patterns.len() - 1 {
                        return Err(anyhow!(
                            "Pattern index out of range {} / {}",
                            index,
                            self.patterns.len()
                        ));
                    }

                    let pattern_cmd = cmd_bytes[2] as char;

                    match pattern_cmd {
                        'c' => self.patterns[index]
                            .0
                            .pattern
                            .execute_command(str::from_utf8(&cmd_bytes[3..]).unwrap())?,
                        's' => self.patterns[index].1 = false,
                        'S' => self.patterns[index].1 = true,
                        'b' => self.patterns[index].2 = false,
                        'B' => self.patterns[index].2 = true,
                        'R' => {
                            self.patterns.remove(index);
                        }
                        'C' => {
                            let (_, (pattern_kind, args)) =
                                pattern_with_args_from_command(&cmd[3..]).map_err(
                                    |err: nom::Err<nom::error::Error<&str>>| {
                                        anyhow!("Could not parse pattern and args! {:?}", err)
                                    },
                                )?;

                            // create the pattern with the given args
                            let pattern: Box<dyn LedPattern> =
                                PatternKind::try_from(pattern_kind)?.to_pattern(args)?;

                            // finally, switch out the new pattern for the old one
                            self.patterns[index].0.pattern = pattern;
                        }
                        c => return Err(anyhow!("Invalid subcommand {}", c)),
                    };
                }
                'g' => (),
                'a' => {
                    let input = str::from_utf8(&cmd_bytes[1..]).unwrap();
                    // adds a new pattern via the Self::add() function
                    // Arguments should be: <n(None)/range char>,patternkind(args for creation)
                    let (remainder, range_str) =
                        take_while(|c: char| c.is_ascii_digit() || c == 'n' || c == '.')(input)
                            .map_err(|_: nom::Err<nom::error::Error<&str>>| {
                                anyhow!("Invalid pattern range. Only 'n' or 'x..y' are allowed")
                            })?;

                    // interpret the range argument
                    let mut range = if range_str == "n" {
                        None
                    } else {
                        let (_, tup) = range_tuple(range_str).map_err(|_| {
                            anyhow!("Invalid pattern range. Only 'n' or 'x..y' are allowed")
                        })?;
                        Some((tup.0 as usize, tup.1 as usize))
                    };

                    let (_, (pattern_kind, args)) = pattern_with_args_from_command(remainder)
                        .map_err(|_: nom::Err<nom::error::Error<&str>>| {
                            anyhow!("Could not parse pattern and args!")
                        })?;

                    // create the pattern with the given args
                    let pattern: Box<dyn LedPattern> =
                        PatternKind::try_from(pattern_kind)?.to_pattern(args)?;

                    if let Some((start, _)) = range.take() {
                        range = Some((start, start + pattern.size()));
                    }

                    // finally, add the pattern with the given range
                    self.add(pattern, range);
                }
                'r' => (),
                c => return Err(anyhow!("Invalid command {} for PartitionedPatterns", c)),
            };
        }

        Ok(())
    }
}

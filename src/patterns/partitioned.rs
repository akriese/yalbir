use alloc::{boxed::Box, vec, vec::Vec};
use anyhow::{anyhow, Error};

use crate::{beat::BeatCount, util::color::Rgb};
use core::str;

use super::{LedPattern, PatternCommand};

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
                        c => return Err(anyhow!("Invalid subcommand {}", c)),
                    };
                }
                'g' => (),
                'a' => (),
                'r' => (),
                c => return Err(anyhow!("Invalid command {} for PartitionedPatterns", c)),
            };
        }

        Ok(())
    }
}

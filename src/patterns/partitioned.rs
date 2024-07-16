use alloc::boxed::Box;

use crate::{beat::BeatCount, util::color::Rgb, N_LEDS};
use core::str;

use super::{LedPattern, PatternCommand};

struct PatternSection {
    range: (usize, usize),
    pattern: Box<dyn LedPattern>,
}

pub struct PartitionedPatterns {
    rgbs: [Rgb; N_LEDS],
    patterns: [Option<PatternSection>; 10],
    status: [bool; 10],
}

impl PartitionedPatterns {
    pub const fn new() -> Self {
        Self {
            rgbs: [Rgb { r: 0, g: 0, b: 0 }; N_LEDS],
            patterns: [None, None, None, None, None, None, None, None, None, None],
            status: [true; 10],
        }
    }

    pub fn add(&mut self, pattern: Box<dyn LedPattern>, range: (usize, usize)) {
        for p in self.patterns.iter_mut() {
            if p.is_some() {
                continue;
            }

            p.replace(PatternSection { pattern, range });
            break;
        }
    }
}

impl LedPattern for PartitionedPatterns {
    fn next(&mut self) -> &[Rgb] {
        for ps in self.patterns.iter_mut() {
            if let Some(section) = ps.as_mut() {
                let rgbs = section.pattern.as_mut().next();
                let (a, b) = (section.range.0, section.range.1);
                self.rgbs[a..b].copy_from_slice(&rgbs[..b - a]);
            }
        }
        &self.rgbs
    }

    fn beat(&mut self, beat_info: &BeatCount) {
        for (ps, status) in self.patterns.iter_mut().zip(self.status) {
            if let Some(section) = ps.as_mut() {
                if status {
                    section.pattern.beat(beat_info);
                }
            }
        }
    }
}

impl PatternCommand for PartitionedPatterns {
    fn execute_command(&mut self, command: &str) -> Result<(), ()> {
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
                    if self.patterns[index].is_none() {
                        return Err(());
                    }

                    let pattern_cmd = cmd_bytes[2] as char;

                    match pattern_cmd {
                        'c' => self.patterns[index]
                            .as_mut()
                            .unwrap()
                            .pattern
                            .execute_command(str::from_utf8(&cmd_bytes[3..]).unwrap())
                            .unwrap(),
                        's' => self.status[index] = false,
                        'r' => self.status[index] = true,
                        'R' => {
                            self.patterns[index] = None;
                            self.status[index] = true;
                        }
                        _ => return Err(()),
                    };
                }
                'g' => (),
                'a' => (),
                'r' => (),
                _ => return Err(()),
            };
        }

        Ok(())
    }
}

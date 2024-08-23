/// Implementation for a remote control shipped with a star projector lamp
///
use crate::control::ir::Actions::{self, *};

use super::RemoteControlVariant;

pub struct SpaceShipRemote {}

impl RemoteControlVariant for SpaceShipRemote {
    fn cmd_to_action(cmd: u8) -> Actions {
        match cmd {
            31 => Power,
            0 => Up,
            8 => Down,
            2 => VolUp,
            10 => VolDown,
            5 => Previous,
            7 => Next,
            6 => Sound,
            4 => Shift,
            11 => Beat,
            12 => Fade,
            14 => Light1,
            15 => Light2,
            13 => LightMixed,
            16 => Mute,
            17 => Button1,
            18 => Button2,
            19 => Button3,
            20 => Time1,
            21 => Time2,
            22 => Time3,
            23 => Time4,
            _ => todo!(),
        }
    }
}

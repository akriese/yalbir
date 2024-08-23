use infrared::protocol::nec::Nec16Command;
use spaceship::SpaceShipRemote;

use super::Actions;

pub mod spaceship;

trait RemoteControlVariant {
    fn cmd_to_action(cmd: u8) -> Actions;
}

pub fn get_action_from_command(cmd: &Nec16Command) -> Option<Actions> {
    match cmd.addr {
        61184 => Some(SpaceShipRemote::cmd_to_action(cmd.cmd)),
        _ => None,
    }
}

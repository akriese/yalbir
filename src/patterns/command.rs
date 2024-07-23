use core::str::FromStr;

use alloc::vec::Vec;

use crate::util::color::Rgb;

pub fn parse_rgb(command: &str) -> Rgb {
    Rgb::from(command)
}

pub fn parse_tuple<T>(command: &str) -> (T, T)
where
    T: FromStr + Copy,
    <T as FromStr>::Err: core::fmt::Debug,
{
    let parts = command.split("..").collect::<Vec<_>>();
    if parts.len() == 1 {
        let single_val = parts[0].parse::<T>().unwrap();
        (single_val, single_val)
    } else {
        (
            parts[0].parse::<T>().unwrap(),
            parts[1].parse::<T>().unwrap(),
        )
    }
}

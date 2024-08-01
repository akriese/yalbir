use anyhow::anyhow;
use core::str::FromStr;

use alloc::vec::Vec;

use crate::util::color::Rgb;

pub fn parse_rgb(command: &str) -> anyhow::Result<Rgb> {
    Rgb::from(command).map_err(|_| anyhow!("Couldnt parse the RGB value!"))
}

pub fn parse<T>(arg: &str) -> anyhow::Result<T>
where
    T: FromStr + Copy,
    <T as FromStr>::Err: core::fmt::Debug,
{
    arg.parse::<T>()
        .map_err(|_| anyhow!("Parsing {:?} went wrong!", arg))
}

pub fn parse_tuple<T>(command: &str) -> anyhow::Result<(T, T)>
where
    T: FromStr + Copy,
    <T as FromStr>::Err: core::fmt::Debug,
{
    let parts = command.split("..").collect::<Vec<_>>();
    if parts.len() == 1 {
        let single_val = parts[0]
            .parse::<T>()
            .map_err(|_| anyhow!("Parsing single value {:?} as tuple went wrong", parts[0]))?;
        Ok((single_val, single_val))
    } else {
        Ok((
            parts[0]
                .parse::<T>()
                .map_err(|_| anyhow!("Parsing first tuple value {:?} went wrong", parts[0]))?,
            parts[1]
                .parse::<T>()
                .map_err(|_| anyhow!("Parsing second tuple value {:?} went wrong", parts[1]))?,
        ))
    }
}

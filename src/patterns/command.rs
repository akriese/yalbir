use anyhow::anyhow;
use core::str::FromStr;
use nom::{
    bytes::complete::{tag, take_while_m_n},
    character::{complete::u32, is_hex_digit},
    error::ErrorKind,
    sequence::pair,
    IResult,
};

use alloc::vec::Vec;

use crate::color::Rgb;

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

pub fn range_tuple(input: &str) -> IResult<&str, (u32, u32)> {
    let (remainder, first) = u32(input)?;

    let output = tag::<_, _, nom::error::Error<&str>>("..")(remainder);
    if output.is_err() {
        return Ok((remainder, (first, first)));
    }

    let (remainder, _) = output.unwrap();

    let (remainder, second) = u32(remainder)?;
    Ok((remainder, (first, second)))
}

pub fn hex_rgb(input: &str) -> IResult<&str, Rgb> {
    let (remainder, (_, rgb_string)) =
        pair(tag("#"), take_while_m_n(6, 6, |c| is_hex_digit(c as u8)))(input)?;

    let rgb = Rgb::from(rgb_string);
    if rgb.is_err() {
        // # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
        Err(nom::Err::Error(nom::error::Error::new(
            "Hex parse error",
            ErrorKind::Fail,
        )))
    } else {
        Ok((remainder, rgb.unwrap()))
    }
}

use bytes::{BufMut, BytesMut};
use std::io;
use std::str;
use std::str::FromStr;
use strum::{Display, EnumString, ParseError};
use tokio_util::codec::{Decoder, Encoder};
use tracing::info;

pub struct Command {
    pub payload: String,
}

impl FromStr for Command {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cmd = Cmd::from_str(s)?;
        let payload = match cmd {
            Cmd::Up => String::from("ss119010000"),
            Cmd::Down => String::from("ss119020000"),
            Cmd::Stop => String::from("ss119000000"),
            Cmd::Init => String::from("init"),
        };
        Ok(Command { payload })
    }
}

#[derive(Debug, Display, EnumString, Eq, PartialEq)]
#[strum(ascii_case_insensitive)]
enum Cmd {
    Stop,
    Up,
    Down,
    Init,
}

pub struct LineCodec;

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let newline = src.as_ref().iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = src.split_to(n + 1);
            return match str::from_utf8(line.as_ref()) {
                Ok(s) => Ok(Some(format!("receiving: {}", s))),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Invalid String")),
            };
        }
        Ok(None)
    }
}

impl Encoder<String> for LineCodec {
    type Error = io::Error;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        info!("sending: {:?}", &item);
        dst.reserve(item.len() + 1);
        dst.put(item.as_bytes());
        dst.put_u8(b'\n');
        Ok(())
    }
}

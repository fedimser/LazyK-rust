use anyhow::Result;
use byteorder::{WriteBytesExt, ReadBytesExt};
use std::io::{BufRead, ErrorKind, Write};

pub enum Input {
    Null,
    Reader(Box<dyn BufRead + 'static>),
}
impl Input {
    pub fn read_byte(&mut self) -> Option<u8> {
        match self {
            Input::Null => None,
            Input::Reader(reader) => match reader.as_mut().read_u8() {
                Ok(ch) => Some(ch),
                Err(err) if err.kind() == ErrorKind::UnexpectedEof => None,
                Err(err) => panic!("I/O error: {}", err),
            },
        }
    }
}

pub enum Output {
    Null,
    Buffer(Vec<u8>),
    Writer(Box<dyn Write + 'static>),
}
impl Output {
    pub fn write_char(&mut self, c: u8) -> Result<()> {
        match self {
            Self::Null => Ok(()),
            Self::Buffer(buf) => {
                buf.push(c);
                Ok(())
            }
            Self::Writer(w) => w.write_u8(c).map_err(anyhow::Error::from),
        }
    }
}

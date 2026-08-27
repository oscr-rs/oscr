use super::parser::{self, Parser};

use core::convert::Infallible;
use core::fmt::{self, Debug, Display};

#[derive(Debug, Clone)]
pub enum Error {
    MissingTagString,
    Parser(parser::Error),
}

impl From<parser::Error> for Error {
    fn from(err: parser::Error) -> Self {
        Self::Parser(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTagString => write!(f, "missing tag string"),
            Self::Parser(e) => write!(f, "parser error: {}", e),
        }
    }
}

impl core::error::Error for Error {}

#[cfg(feature = "parse")]
pub trait Parse<'a>: Sized {
    type Error;
    fn parse(parser: &'a mut Parser) -> Result<Self, Self::Error>;
}

#[cfg(feature = "serialize")]
pub trait Serialize {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error>;
    fn len(&self) -> usize {
        let mut noop = NoopWriter::default();
        self.serialize(&mut noop);
        noop.0
    }
}

#[cfg(feature = "serialize")]
pub trait Write: Sized {
    type Error;
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    fn write_u8(&mut self, value: u8) -> Result<(), Self::Error> {
        self.write(&[value])
    }

    fn write_be_i32(&mut self, value: i32) -> Result<(), Self::Error> {
        self.write(&value.to_be_bytes())
    }

    fn write_be_u32(&mut self, value: u32) -> Result<(), Self::Error> {
        self.write(&value.to_be_bytes())
    }

    fn write_be_i64(&mut self, value: i64) -> Result<(), Self::Error> {
        self.write(&value.to_be_bytes())
    }

    fn write_be_u64(&mut self, value: u64) -> Result<(), Self::Error> {
        self.write(&value.to_be_bytes())
    }

    fn write_be_f32(&mut self, value: f32) -> Result<(), Self::Error> {
        self.write(&value.to_be_bytes())
    }

    fn write_be_f64(&mut self, value: f64) -> Result<(), Self::Error> {
        self.write(&value.to_be_bytes())
    }

    fn write_padding(&mut self, len: usize) -> Result<(), Self::Error> {
        let pad = (4 - (len % 4)) % 4;
        if pad > 0 {
            static ZEROS: [u8; 3] = [0, 0, 0];
            self.write(&ZEROS[..pad])?;
        }
        Ok(())
    }
}

#[cfg(feature = "serialize")]
#[derive(Debug)]
pub struct BufferTooShort(());

#[cfg(feature = "serialize")]
impl Display for BufferTooShort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "buffer too short")
    }
}

#[cfg(feature = "serialize")]
impl Write for &mut [u8] {
    type Error = BufferTooShort;
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        if bytes.len() > self.len() {
            return Err(BufferTooShort(()));
        }
        let (first, rest) = core::mem::take(self).split_at_mut(bytes.len());
        first.copy_from_slice(bytes);
        *self = rest;
        Ok(())
    }
}

#[cfg(feature = "serialize")]
#[derive(Debug, Default)]
struct NoopWriter(usize);

#[cfg(feature = "serialize")]
impl Write for NoopWriter {
    type Error = Infallible;

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0 += bytes.len();
        Ok(())
    }
}

#[cfg(all(feature = "serialize", feature = "alloc"))]
impl Write for Vec<u8> {
    type Error = Infallible;
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

#[cfg(feature = "alloc")]
pub fn to_bytes<T: Serialize>(data: T) -> Vec<u8> {
    let mut v = Vec::new();
    data.serialize(&mut v);
    v
}

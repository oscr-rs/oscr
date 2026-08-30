use super::address;
use super::arg::TagError;
use super::parser;
#[cfg(feature = "parse")]
use super::parser::Parser;
use super::zstr::ZStr;

#[cfg(feature = "serialize")]
use core::convert::Infallible;
use core::fmt::{self, Debug, Display};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub enum Error {
    Address(address::Error),
    Tag(TagError),
    TagString,
    Packet(Option<u8>),
    Char(u32),
    Parser(parser::Error),
}

impl From<address::Error> for Error {
    fn from(err: address::Error) -> Self {
        Self::Address(err)
    }
}

impl From<TagError> for Error {
    fn from(err: TagError) -> Self {
        Self::Tag(err)
    }
}

impl From<parser::Error> for Error {
    fn from(err: parser::Error) -> Self {
        Self::Parser(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address(e) => write!(f, "cannot parse address: {}", e),
            Self::Tag(e) => write!(f, "cannot parse tag: {}", e),
            Self::TagString => write!(f, "cannot find tag string"),
            Self::Packet(byte) => match byte {
                Some(b) if b.is_ascii_graphic() => {
                    write!(
                        f,
                        "cannot parse packet: invalid packet magic '{}'",
                        *b as char
                    )
                }
                Some(b) => {
                    write!(
                        f,
                        "cannot parse packet: invalid packet magic '\\x{:02x}'",
                        b
                    )
                }
                None => write!(f, "cannot find packet magic"),
            },
            Self::Char(c) => write!(f, "invalid char {:#08x}", c),
            Self::Parser(e) => write!(f, "cannot parse: {}", e),
        }
    }
}

impl core::error::Error for Error {}

#[cfg(any(feature = "parse", feature = "serialize"))]
pub(super) fn padding(len: usize) -> &'static [u8] {
    const ZEROS: [u8; 3] = [0, 0, 0];
    let pad = (4 - (len % 4)) % 4;
    if pad > 0 {
        &ZEROS[..pad]
    } else {
        &[]
    }
}

#[cfg(feature = "parse")]
pub trait Parse<'a>: Sized {
    type Error;
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Self::Error>;
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
        self.write(padding(len))
    }

    fn write_zstr_padded(&mut self, zstr: &ZStr) -> Result<(), Self::Error> {
        self.write(zstr.as_bytes())?;
        self.write_u8(0u8)?;
        self.write(padding(zstr.len() + 1))
    }

    fn write_padded(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write(bytes)?;
        self.write(padding(bytes.len()))
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

#[cfg(all(feature = "parse"))]
use super::packet::{MessageRef, PacketRef};

#[cfg(all(feature = "parse"))]
pub fn parse_packet(bytes: &[u8]) -> Result<PacketRef<'_>, Error> {
    let mut parser = Parser::new(bytes);
    PacketRef::parse(&mut parser)
}

#[cfg(all(feature = "parse"))]
pub fn parse_message(bytes: &[u8]) -> Result<MessageRef<'_>, Error> {
    let mut parser = Parser::new(bytes);
    MessageRef::parse(&mut parser)
}

#[cfg(all(feature = "serialize", feature = "alloc"))]
pub fn to_bytes<T: Serialize>(data: T) -> Vec<u8> {
    let mut v = Vec::new();
    data.serialize(&mut v);
    v
}

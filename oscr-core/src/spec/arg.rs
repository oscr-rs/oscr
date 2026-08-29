use super::macros::{define_owned_and_ref, define_tags, impl_both};
use super::time::TimeTag;
use super::zstr::*;

#[cfg(feature = "parse")]
use super::parser::Parser;
#[cfg(feature = "parse")]
use super::wire;
#[cfg(feature = "serialize")]
use super::wire::{Serialize, Write};

use core::fmt::{self, Display};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct TagError(u8);

impl Display for TagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_ascii_graphic() {
            write!(f, "unsupported tag '{}'", self.0 as char)
        } else {
            write!(f, "unsupported tag '\\x{:02x}'", self.0)
        }
    }
}

define_tags! {
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Tag {
        // OSC 1.0 required types
        // https://opensoundcontrol.stanford.edu/spec-1_0.html#osc-type-tag-string
        Int32 = b'i',
        Float = b'f',
        String = b's',
        Blob = b'b',

        // OSC 1.1 additional required types
        // https://ccrma.stanford.edu/groups/osc/files/2009-NIME-OSC-1.1.pdf
        True = b'T',
        False = b'F',
        Null = b'N',
        Impulse = b'I',
        TimeTag = b't',

        // OSC 1.0 additional types
        // https://opensoundcontrol.stanford.edu/spec-1_0.html#:~:text=certain%20nonstandard%20argument%20types
        Int64 = b'h',
        Double = b'd',
        AlternateString = b'S',
        Char = b'c',
        Rgba = b'r',
        Midi = b'm',
        ArrayStart = b'[',
        ArrayEnd = b']',
    }
}

impl Tag {
    pub fn as_byte(&self) -> u8 {
        *self as _
    }
}

impl TryFrom<u8> for Tag {
    type Error = TagError;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Self::from_byte(byte).ok_or(TagError(byte))
    }
}

impl Into<u8> for Tag {
    fn into(self) -> u8 {
        self.as_byte()
    }
}

define_owned_and_ref! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Arg => ArgRef<'a> {
        // OSC 1.0 required types
        Int32(i32 => i32),
        Float(f32 => f32),
        String(ZString => &'a ZStr),
        Blob(Vec<u8> => &'a [u8]),
        // OSC 1.1 additional required types
        True,
        False,
        Null,
        Impulse,
        TimeTag(TimeTag => TimeTag),
        // OSC 1.0 additional types
        Int64(i64 => i64),
        Double(f64 => f64),
        AlternateString(ZString => &'a ZStr),
        Char(char => char),
        Rgba([u8; 4] => [u8; 4]),
        Midi([u8; 4] => [u8; 4]),
        ArrayStart,
        ArrayEnd,
    }
}

impl Copy for ArgRef<'_> {}

#[cfg(feature = "alloc")]
impl PartialEq<ArgRef<'_>> for Arg {
    fn eq(&self, other: &ArgRef<'_>) -> bool {
        match (self, other) {
            // OSC 1.0 required types
            (Self::Int32(l), ArgRef::Int32(r)) => l == r,
            (Self::Float(l), ArgRef::Float(r)) => l == r,
            (Self::String(l), ArgRef::String(r)) => l.as_zstr() == *r,
            (Self::Blob(l), ArgRef::Blob(r)) => l == r,
            // OSC 1.1 additional required types
            (Self::True, ArgRef::True) => true,
            (Self::False, ArgRef::False) => true,
            (Self::Null, ArgRef::Null) => true,
            (Self::Impulse, ArgRef::Impulse) => true,
            (Self::TimeTag(l), ArgRef::TimeTag(r)) => l == r,
            // OSC 1.0 additional types
            (Self::Int64(l), ArgRef::Int64(r)) => l == r,
            (Self::Double(l), ArgRef::Double(r)) => l == r,
            (Self::AlternateString(l), ArgRef::AlternateString(r)) => l.as_zstr() == *r,
            (Self::Char(l), ArgRef::Char(r)) => l == r,
            (Self::Rgba(l), ArgRef::Rgba(r)) => l == r,
            (Self::Midi(l), ArgRef::Midi(r)) => l == r,
            (Self::ArrayStart, ArgRef::ArrayStart) => true,
            (Self::ArrayEnd, ArgRef::ArrayEnd) => true,
            _ => false,
        }
    }
}

impl_both! {
    impl Arg => ArgRef<'_> {
        pub fn tag(&self) -> Tag {
            match self {
                // OSC 1.0 required types
                Self::Int32(..) => Tag::Int32,
                Self::Float(..) => Tag::Float,
                Self::String(..) => Tag::String,
                Self::Blob(..) => Tag::Blob,
                // OSC 1.1 additional required types
                Self::True => Tag::True,
                Self::False => Tag::False,
                Self::Null => Tag::Null,
                Self::Impulse => Tag::Impulse,
                Self::TimeTag(..) => Tag::TimeTag,
                // OSC 1.0 additional types
                Self::Int64(..) => Tag::Int64,
                Self::Double(..) => Tag::Double,
                Self::AlternateString(..) => Tag::AlternateString,
                Self::Char(..) => Tag::Char,
                Self::Rgba(..) => Tag::Rgba,
                Self::Midi(..) => Tag::Midi,
                Self::ArrayStart => Tag::ArrayStart,
                Self::ArrayEnd => Tag::ArrayEnd,
            }
        }
    }
}

#[cfg(feature = "alloc")]
impl ArgRef<'_> {
    pub fn to_owned(&self) -> Arg {
        match self {
            // OSC 1.0 required types
            Self::Int32(int) => Arg::Int32(*int),
            Self::Float(float) => Arg::Float(*float),
            Self::String(s) => Arg::String(s.to_zstring()),
            Self::Blob(data) => Arg::Blob(data.to_vec()),
            // OSC 1.1 additional required types
            Self::True => Arg::True,
            Self::False => Arg::False,
            Self::Null => Arg::Null,
            Self::Impulse => Arg::Impulse,
            Self::TimeTag(t) => Arg::TimeTag(*t),
            // OSC 1.0 additional types
            Self::Int64(int) => Arg::Int64(*int),
            Self::Double(double) => Arg::Double(*double),
            Self::AlternateString(s) => Arg::AlternateString(s.to_zstring()),
            Self::Char(c) => Arg::Char(*c),
            Self::Rgba(rgba) => Arg::Rgba(*rgba),
            Self::Midi(midi) => Arg::Midi(*midi),
            Self::ArrayStart => Arg::ArrayStart,
            Self::ArrayEnd => Arg::ArrayEnd,
        }
    }
}

#[cfg(feature = "parse")]
impl<'a> ArgRef<'a> {
    pub fn parse_tag(parser: &mut Parser<'a>, tag: Tag) -> Result<Self, wire::Error> {
        match tag {
            // OSC 1.0 required types
            Tag::Int32 => Ok(parser.take_be_i32().map(Self::Int32)?),
            Tag::Float => Ok(parser.take_be_f32().map(Self::Float)?),
            Tag::String => Ok(parser.take_zstr_padded().map(Self::String)?),
            Tag::Blob => {
                let len = parser.take_be_i32()?;
                let data = parser.take_padded(len as _)?;
                Ok(Self::Blob(data))
            }
            // OSC 1.1 additional required types
            Tag::True => Ok(Self::True),
            Tag::False => Ok(Self::False),
            Tag::Null => Ok(Self::Null),
            Tag::Impulse => Ok(Self::Impulse),
            Tag::TimeTag => {
                let t = parser.take_be_u64().map(TimeTag::from_raw)?;
                Ok(Self::TimeTag(t))
            }
            // OSC 1.0 additional types
            Tag::Int64 => Ok(parser.take_be_i64().map(Self::Int64)?),
            Tag::Double => Ok(parser.take_be_f64().map(Self::Double)?),
            Tag::AlternateString => Ok(parser.take_zstr_padded().map(Self::AlternateString)?),
            Tag::Char => {
                let c = parser.take_be_u32()?;
                let c = char::from_u32(c).ok_or(wire::Error::Char(c))?;
                Ok(Self::Char(c))
            }
            Tag::Rgba => Ok(parser.take_array::<4>().map(Self::Rgba)?),
            Tag::Midi => Ok(parser.take_array::<4>().map(Self::Midi)?),
            Tag::ArrayStart => Ok(Self::ArrayStart),
            Tag::ArrayEnd => Ok(Self::ArrayEnd),
        }
    }
}

#[cfg(feature = "serialize")]
impl_both! {
    impl(Serialize) Arg => ArgRef<'_> {
        fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
            match self {
                // OSC 1.0 required types
                Self::Int32(int) => w.write_be_i32(*int),
                Self::Float(float) => w.write_be_f32(*float),
                Self::String(s) => {
                    w.write(s.as_bytes())?;
                    w.write_u8(0)?;
                    w.write_padding(s.len() + 1)
                },
                Self::Blob(data) => {
                    w.write_be_i32(data.len() as _)?;
                    w.write(&data)?;
                    w.write_padding(data.len())
                },
                // OSC 1.1 additional required types
                Self::True | Self::False | Self::Null | Self::Impulse => Ok(()),
                Self::TimeTag(t) => t.serialize(w),
                // OSC 1.0 additional types
                Self::Int64(int) => w.write_be_i64(*int),
                Self::Double(double) => w.write_be_f64(*double),
                Self::AlternateString(s) => {
                    w.write(s.as_bytes())?;
                    w.write_u8(0)?;
                    w.write_padding(s.len() + 1)
                },
                Self::Char(c) => {
                    w.write_be_u32(*c as u32)
                },
                Self::Rgba(rgba) => {
                    w.write(rgba.as_slice())
                },
                Self::Midi(midi) => {
                    w.write(midi.as_slice())
                },
                Self::ArrayStart => Ok(()),
                Self::ArrayEnd => Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_as_byte() {
        // OSC 1.0 required types
        assert_eq!(Tag::Int32.as_byte(), b'i');
        assert_eq!(Tag::Float.as_byte(), b'f');
        assert_eq!(Tag::String.as_byte(), b's');
        assert_eq!(Tag::Blob.as_byte(), b'b');

        // OSC 1.1 additional required types
        assert_eq!(Tag::True.as_byte(), b'T');
        assert_eq!(Tag::False.as_byte(), b'F');
        assert_eq!(Tag::Null.as_byte(), b'N');
        assert_eq!(Tag::Impulse.as_byte(), b'I');
        assert_eq!(Tag::TimeTag.as_byte(), b't');

        // OSC 1.0 additional types
        assert_eq!(Tag::Int64.as_byte(), b'h');
        assert_eq!(Tag::Double.as_byte(), b'd');
        assert_eq!(Tag::AlternateString.as_byte(), b'S');
        assert_eq!(Tag::Char.as_byte(), b'c');
        assert_eq!(Tag::Rgba.as_byte(), b'r');
        assert_eq!(Tag::Midi.as_byte(), b'm');
        assert_eq!(Tag::ArrayStart.as_byte(), b'[');
        assert_eq!(Tag::ArrayEnd.as_byte(), b']');
    }
}

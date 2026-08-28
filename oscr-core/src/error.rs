use core::fmt::{self, Display};

use crate::wire;

#[derive(Debug)]
pub enum Error {
    Tag(u8),
    Address(Option<u8>),
    Wire(wire::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag(byte) => {
                if byte.is_ascii_graphic() {
                    write!(f, "unsupported tag '{}'", *byte as char)
                } else {
                    write!(f, "unsupported tag '\\x{:02x}'", byte)
                }
            }
            Self::Address(byte) => match byte {
                Some(b) if b.is_ascii_graphic() => {
                    write!(f, "invalid address byte '{}'", *b as char)
                }
                Some(b) => {
                    write!(f, "invalid address byte '\\x{:02x}'", b)
                }
                None => write!(f, "missing address magic"),
            },
            Self::Wire(e) => write!(f, "wire error: {}", e),
        }
    }
}

impl core::error::Error for Error {}

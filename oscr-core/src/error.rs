use core::fmt::{self, Display};

use crate::wire;

#[derive(Debug)]
pub enum Error {
    UnsupportedTag(u8),
    InvalidAddress(u8),
    Wire(wire::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTag(byte) => {
                if byte.is_ascii_graphic() {
                    write!(f, "unsupported tag '{}'", *byte as char)
                } else {
                    write!(f, "unsupported tag '\\x{:02x}'", byte)
                }
            }
            Self::InvalidAddress(byte) => {
                if byte.is_ascii_graphic() {
                    write!(f, "invalid address byte '{}'", *byte as char)
                } else {
                    write!(f, "invalid address byte '\\x{:02x}'", byte)
                }
            }
            Self::Wire(e) => write!(f, "wire error: {}", e),
        }
    }
}

impl core::error::Error for Error {}

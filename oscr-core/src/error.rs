use core::fmt::{self, Display};

use crate::spec::address;
use crate::spec::arg::TagError;

use crate::wire;

#[derive(Debug)]
pub enum Error {
    Tag(TagError),
    Address(address::Error),
    Wire(wire::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag(e) => write!(f, "tag error: {}", e),
            Self::Address(e) => write!(f, "address error: {}", e),
            Self::Wire(e) => write!(f, "wire {}", e),
        }
    }
}

impl core::error::Error for Error {}

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

impl From<TagError> for Error {
    fn from(err: TagError) -> Self {
        Self::Tag(err)
    }
}

impl From<address::Error> for Error {
    fn from(err: address::Error) -> Self {
        Self::Address(err)
    }
}

impl From<wire::Error> for Error {
    fn from(err: wire::Error) -> Self {
        Self::Wire(err)
    }
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

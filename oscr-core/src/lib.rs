#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod error;
mod spec;

pub use error::Error;
pub use spec::address::*;
pub use spec::arg::*;
pub use spec::packet::*;
pub use spec::time::*;
pub use spec::wire;
pub use spec::zstr::*;

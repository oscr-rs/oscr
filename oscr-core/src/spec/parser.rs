use super::zstr::ZStr;

use core::fmt::{self, Display};

#[derive(Debug, Clone)]
pub enum Error {
    Peek {
        position: usize,
    },
    Advance {
        position: usize,
        step: usize,
        remaining: usize,
    },
    UnexpectedEof {
        position: usize,
        need: usize,
        remaining: usize,
    },
    ZStr {
        position: usize,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Peek { position } => write!(f, "cannot peek at position {}", position),
            Self::Advance {
                position,
                step,
                remaining,
            } => write!(
                f,
                "cannot advance {} at position {}, remaining {}",
                step, position, remaining
            ),
            Self::UnexpectedEof {
                position,
                need,
                remaining,
            } => write!(
                f,
                "unexpected eof at position {}, need {}, remaining {}",
                position, need, remaining
            ),
            Self::ZStr { position } => write!(f, "cannot find zstr at position {}", position),
        }
    }
}

impl core::error::Error for Error {}

#[cfg(feature = "parse")]
#[derive(Debug, Clone)]
pub struct Parser<'a> {
    original: &'a [u8],
    view: &'a [u8],
}

#[cfg(feature = "parse")]
impl<'a> Parser<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            original: bytes,
            view: bytes,
        }
    }

    pub fn eof(&self) -> bool {
        self.view.is_empty()
    }

    pub fn position(&self) -> usize {
        unsafe {
            self.view
                .as_ptr()
                .offset_from_unsigned(self.original.as_ptr())
        }
    }

    pub fn peek(&self) -> Result<u8, Error> {
        self.view.first().copied().ok_or(Error::Peek {
            position: self.position(),
        })
    }

    pub fn remaining(&self) -> &'a [u8] {
        self.view
    }

    pub fn advance(&mut self, step: usize) -> Result<(), Error> {
        let (_, view) = self.view.split_at_checked(step).ok_or(Error::Advance {
            position: self.position(),
            step,
            remaining: self.view.len(),
        })?;
        self.view = view;
        Ok(())
    }

    pub fn take_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let (taken, view) = self.view.split_at_checked(N).ok_or(Error::UnexpectedEof {
            position: self.position(),
            need: N,
            remaining: self.view.len(),
        })?;
        self.view = view;
        Ok(taken.try_into().unwrap())
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let (taken, view) = self
            .view
            .split_at_checked(len)
            .ok_or(Error::UnexpectedEof {
                position: self.position(),
                need: len,
                remaining: self.view.len(),
            })?;
        self.view = view;
        Ok(taken)
    }

    pub fn take_be_i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_be_bytes(self.take_array::<4>()?))
    }

    pub fn take_be_u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.take_array::<4>()?))
    }

    pub fn take_be_i64(&mut self) -> Result<i64, Error> {
        Ok(i64::from_be_bytes(self.take_array::<8>()?))
    }

    pub fn take_be_u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.take_array::<8>()?))
    }

    pub fn take_be_f32(&mut self) -> Result<f32, Error> {
        Ok(f32::from_be_bytes(self.take_array::<4>()?))
    }

    pub fn take_be_f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_be_bytes(self.take_array::<8>()?))
    }

    pub fn take_zstr_padded(&mut self) -> Result<&'a ZStr, Error> {
        use super::wire::padding;
        let zpos = self.view.iter().position(|&b| b == 0).ok_or(Error::ZStr {
            position: self.position(),
        })?;
        let bytes = &self.view[..zpos];
        let len = zpos + 1;
        let skip = len + padding(len).len();
        self.advance(skip)?;
        Ok(unsafe { ZStr::from_bytes_unchecked(bytes) })
    }

    pub fn take_padded(&mut self, len: usize) -> Result<&'a [u8], Error> {
        use super::wire::padding;
        let bytes = self.take(len)?;
        let skip = padding(len).len();
        self.advance(skip)?;
        Ok(bytes)
    }
}

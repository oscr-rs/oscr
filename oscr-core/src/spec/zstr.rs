use super::macros::define_owned_and_ref;

use core::fmt::{self, Display};
use core::str::Utf8Error;

#[cfg(feature = "alloc")]
use alloc::borrow::{Borrow, Cow, ToOwned};
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

define_owned_and_ref! {
    #[repr(transparent)]
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ZString => ZStr(Vec<u8> => [u8]);
}

#[cfg(feature = "alloc")]
impl Clone for ZString {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg(feature = "alloc")]
impl Default for ZString {
    fn default() -> Self {
        Self(Vec::new())
    }
}

#[derive(Debug)]
pub struct NulError {
    position: usize,
}

impl NulError {
    pub fn position(&self) -> usize {
        self.position
    }
}

impl Display for NulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unexpected NUL byte found at {}", self.position)
    }
}

impl core::error::Error for NulError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromBytesWithNulError {
    InteriorNul { position: usize },
    NotNulTerminated,
}

impl Display for FromBytesWithNulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InteriorNul { position } => {
                write!(f, "unexpected interior NUL byte found at {}", position)
            }
            Self::NotNulTerminated => write!(f, "bytes not NUL terminated"),
        }
    }
}

impl core::error::Error for FromBytesWithNulError {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FromBytesUntilNulError(());

impl fmt::Display for FromBytesUntilNulError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NUL byte not found")
    }
}

impl core::error::Error for FromBytesUntilNulError {}

impl ZStr {
    pub fn new<S: AsRef<[u8]> + ?Sized>(s: &S) -> &Self {
        Self::from_bytes_lossy(s.as_ref())
    }

    pub const fn from_bytes_lossy(bytes: &[u8]) -> &Self {
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0u8 {
                let slice = unsafe { core::slice::from_raw_parts(bytes.as_ptr(), i) };
                return unsafe { Self::from_bytes_unchecked(slice) };
            }
            i += 1;
        }
        unsafe { Self::from_bytes_unchecked(&bytes) }
    }

    /// Constructs a [`ZStr`] with non-zero bytes.
    pub const fn from_bytes(bytes: &[u8]) -> Result<&Self, NulError> {
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0u8 {
                return Err(NulError { position: i });
            }
            i += 1;
        }
        Ok(unsafe { Self::from_bytes_unchecked(bytes) })
    }

    pub const unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes as *const [u8] as *const Self) }
    }

    /// Similar to [`core::ffi::CStr::from_bytes_with_nul`].
    pub const fn from_bytes_with_nul(bytes: &[u8]) -> Result<&Self, FromBytesWithNulError> {
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0u8 {
                if i + 1 == bytes.len() {
                    let slice = unsafe { core::slice::from_raw_parts(bytes.as_ptr(), i) };
                    return Ok(unsafe { Self::from_bytes_unchecked(slice) });
                } else {
                    return Err(FromBytesWithNulError::InteriorNul { position: i });
                }
            }
            i += 1;
        }
        Err(FromBytesWithNulError::NotNulTerminated)
    }

    pub const fn from_bytes_until_nul(bytes: &[u8]) -> Result<&Self, FromBytesUntilNulError> {
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0u8 {
                let slice = unsafe { core::slice::from_raw_parts(bytes.as_ptr(), i) };
                return Ok(unsafe { Self::from_bytes_unchecked(slice) });
            }
            i += 1;
        }
        Err(FromBytesUntilNulError(()))
    }
}

impl<'a> From<&'a str> for &'a ZStr {
    fn from(s: &'a str) -> Self {
        ZStr::new(s)
    }
}

impl<'a> From<&'a [u8]> for &'a ZStr {
    fn from(s: &'a [u8]) -> Self {
        ZStr::new(s)
    }
}

impl AsRef<ZStr> for ZStr {
    fn as_ref(&self) -> &ZStr {
        self
    }
}

impl AsRef<ZStr> for str {
    fn as_ref(&self) -> &ZStr {
        ZStr::new(self)
    }
}

impl AsRef<ZStr> for [u8] {
    fn as_ref(&self) -> &ZStr {
        ZStr::new(self)
    }
}

impl<const N: usize> AsRef<ZStr> for [u8; N] {
    fn as_ref(&self) -> &ZStr {
        ZStr::new(self)
    }
}

#[cfg(feature = "alloc")]
impl AsRef<ZStr> for Vec<u8> {
    fn as_ref(&self) -> &ZStr {
        ZStr::new(self)
    }
}

#[cfg(feature = "alloc")]
impl ZString {
    pub fn new<S: Into<Vec<u8>>>(s: S) -> Self {
        Self::from_vec_lossy(s.into())
    }

    pub fn from_vec_lossy(mut v: Vec<u8>) -> Self {
        if let Some(position) = v.iter().position(|b| b == &0u8) {
            unsafe { v.set_len(position) }
        }
        Self(v)
    }

    pub fn from_vec(v: Vec<u8>) -> Result<Self, NulError> {
        if let Some(position) = v.iter().position(|b| b == &0u8) {
            Err(NulError { position })
        } else {
            Ok(Self(v))
        }
    }

    #[inline]
    pub const unsafe fn from_vec_unchecked(v: Vec<u8>) -> Self {
        Self(v)
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    #[inline]
    pub const fn as_zstr(&self) -> &ZStr {
        unsafe { ZStr::from_bytes_unchecked(self.0.as_slice()) }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    pub fn push_zstr(&mut self, zstr: &ZStr) {
        self.0.extend_from_slice(zstr.as_bytes());
    }

    #[inline]
    pub fn push(&mut self, ch: u8) {
        self.0.push(ch);
    }
}

#[cfg(feature = "alloc")]
impl core::ops::Deref for ZString {
    type Target = ZStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { ZStr::from_bytes_unchecked(&self.0) }
    }
}

#[cfg(feature = "alloc")]
impl AsRef<ZStr> for ZString {
    #[inline]
    fn as_ref(&self) -> &ZStr {
        unsafe { ZStr::from_bytes_unchecked(&self.0) }
    }
}

#[cfg(feature = "alloc")]
impl From<&str> for ZString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[cfg(feature = "alloc")]
impl From<&mut str> for ZString {
    fn from(s: &mut str) -> Self {
        s.to_owned().into()
    }
}

#[cfg(feature = "alloc")]
impl From<String> for ZString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[cfg(feature = "alloc")]
impl From<&String> for ZString {
    fn from(s: &String) -> Self {
        Self::new(s.as_str())
    }
}

impl ZStr {
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[inline]
    pub fn strip_prefix(&self, prefix: u8) -> Option<&Self> {
        let stripped = self.0.strip_prefix(&[prefix])?;
        Some(unsafe { Self::from_bytes_unchecked(stripped) })
    }

    #[cfg(feature = "alloc")]
    pub fn to_zstring(&self) -> ZString {
        unsafe { ZString::from_vec_unchecked(self.0.to_vec()) }
    }

    #[inline]
    pub const fn to_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(&self.0)
    }

    #[inline]
    pub const unsafe fn to_str_unchecked(&self) -> &str {
        str::from_utf8_unchecked(&self.0)
    }

    #[cfg(feature = "alloc")]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }
}

#[cfg(feature = "alloc")]
impl Borrow<ZStr> for ZString {
    #[inline]
    fn borrow(&self) -> &ZStr {
        unsafe { ZStr::from_bytes_unchecked(&self.0) }
    }
}

#[cfg(feature = "alloc")]
impl ToOwned for ZStr {
    type Owned = ZString;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        unsafe { ZString::from_vec_unchecked(self.0.to_owned()) }
    }

    #[inline]
    fn clone_into(&self, target: &mut Self::Owned) {
        target.clear();
        target.push_zstr(self);
    }
}

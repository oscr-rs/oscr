use super::macros::define_owned_and_ref;
use super::zstr;
use super::zstr::*;

#[cfg(feature = "parse")]
use super::parser::Parser;
#[cfg(feature = "parse")]
use super::wire::{self, Parse};

#[cfg(feature = "serialize")]
use super::macros::impl_both;
#[cfg(feature = "serialize")]
use super::wire::{Serialize, Write};

use core::fmt;
use core::str::FromStr;

#[cfg(feature = "alloc")]
use alloc::borrow::{Borrow, ToOwned};
#[cfg(feature = "alloc")]
use core::ops::Deref;

pub struct Display<'a> {
    inner: zstr::Display<'a>,
}

impl fmt::Debug for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl fmt::Display for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

#[derive(Debug, Clone)]
pub struct MagicError(pub(super) Option<u8>);

impl fmt::Display for MagicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(b'/') => write!(f, "trailing slash"),
            Some(b) if b.is_ascii_graphic() => write!(f, "invalid magic '{}'", b as char),
            Some(b) => write!(f, "invalid magic '\\x{:02x}'", b),
            None => write!(f, "missing magic"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InvalidByte {
    pub(super) position: usize,
    pub(super) byte: u8,
}

impl InvalidByte {
    pub const fn shift(&mut self, offset: isize) {
        self.position = self.position.saturating_add_signed(offset);
    }

    pub const fn position(&self) -> usize {
        self.position
    }

    pub const fn byte(&self) -> u8 {
        self.byte
    }
}

impl fmt::Display for InvalidByte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let byte = self.byte;
        if byte.is_ascii_graphic() {
            let c = byte as char;
            write!(f, "invalid byte '{}' at position {}", c, self.position)
        } else {
            write!(
                f,
                "invalid byte '\\x{:02x}' at position {}",
                byte, self.position
            )
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Error {
    Magic(MagicError),
    Invalid(InvalidByte),
    Slashes(usize),
    Trailing,
    #[cfg(feature = "pattern")]
    Validation(super::pattern::ValidatorError),
}

impl From<MagicError> for Error {
    fn from(err: MagicError) -> Self {
        Self::Magic(err)
    }
}

impl From<InvalidByte> for Error {
    fn from(err: InvalidByte) -> Self {
        Self::Invalid(err)
    }
}

#[cfg(feature = "pattern")]
impl From<super::pattern::ValidatorError> for Error {
    fn from(err: super::pattern::ValidatorError) -> Self {
        Self::Validation(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Magic(e) => e.fmt(f),
            Self::Invalid(e) => e.fmt(f),
            Self::Slashes(n) => write!(f, "too many slashes ({})", n),
            Self::Trailing => write!(f, "trailing slash"),
            #[cfg(feature = "pattern")]
            Self::Validation(e) => e.fmt(f),
        }
    }
}

impl core::error::Error for Error {}

const fn charset_map(charset: &[u8]) -> [bool; 256] {
    let mut table = [false; 256];
    let mut i = 0;
    while i < charset.len() {
        table[charset[i] as usize] = true;
        i += 1;
    }
    table
}

pub(super) const SEPARATOR: u8 = b'/';
pub(super) const DISALLOWED: &[u8] = b" #*,?[]{}";
pub(super) const DISALLOWED_MAP: [bool; 256] = charset_map(DISALLOWED);
pub(super) const SEGMENT_DISALLOWED: &[u8] = b" #*,/?[]{}";
pub(super) const SEGMENT_DISALLOWED_MAP: [bool; 256] = charset_map(SEGMENT_DISALLOWED);

#[inline]
pub(super) const fn is_byte_disallowed(byte: u8) -> bool {
    #[cfg(feature = "lut_address_check")]
    {
        DISALLOWED_MAP[byte as usize]
    }
    #[cfg(not(feature = "lut_address_check"))]
    {
        matches!(
            byte,
            b' ' | b'#' | b'*' | b',' | b'?' | b'[' | b']' | b'{' | b'}'
        )
    }
}

#[inline]
pub(super) const fn is_byte_disallowed_segment(byte: u8) -> bool {
    #[cfg(feature = "lut_address_check")]
    {
        SEGMENT_DISALLOWED_MAP[byte as usize]
    }
    #[cfg(not(feature = "lut_address_check"))]
    {
        matches!(
            byte,
            b' ' | b'#' | b'*' | b',' | b'/' | b'?' | b'[' | b']' | b'{' | b'}'
        )
    }
}

pub(super) const fn check_magic(bytes: &[u8]) -> Result<&[u8], MagicError> {
    if let Some(&first) = bytes.first() {
        if first != SEPARATOR {
            Err(MagicError(Some(first)))
        } else {
            if let Some((&last, trim)) = bytes.split_last() {
                if last == SEPARATOR {
                    match trim.last() {
                        Some(&b) if b != SEPARATOR => {
                            #[cfg(feature = "compat_trailing_slash")]
                            {
                                Ok(trim)
                            }
                            #[cfg(not(feature = "compat_trailing_slash"))]
                            {
                                Err(MagicError(Some(b'/')))
                            }
                        }
                        _ => Ok(bytes),
                    }
                } else {
                    Ok(bytes)
                }
            } else {
                Ok(bytes)
            }
        }
    } else {
        Err(MagicError(None))
    }
}

pub(super) const fn check_charset(bytes: &[u8]) -> Result<(), Error> {
    let len = bytes.len();
    let mut i = 0;
    let mut slashes = 0;

    while i < len {
        let byte = bytes[i];

        if byte == b'/' {
            slashes += 1
        } else {
            slashes = 0;
        }

        if slashes > 1 {
            return Err(Error::Slashes(slashes));
        }

        if is_byte_disallowed(byte) {
            return Err(Error::Invalid(InvalidByte { position: i, byte }));
        }

        i += 1;
    }

    Ok(())
}

pub(super) const fn check_segment(bytes: &[u8]) -> Result<(), InvalidByte> {
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let byte = bytes[i];
        if is_byte_disallowed_segment(byte) {
            return Err(InvalidByte { position: i, byte });
        }
        i += 1;
    }

    Ok(())
}

#[inline]
pub(super) const fn check(bytes: &[u8]) -> Result<(), Error> {
    match check_magic(bytes) {
        Ok(_) => match check_charset(bytes) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
        Err(e) => Err(Error::Magic(e)),
    }
}

define_owned_and_ref! {
    #[
        derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash) =>
        derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)
    ]
    pub struct AddressBuf => Address(ZString => ZStr);
}

#[cfg(feature = "alloc")]
impl Default for AddressBuf {
    fn default() -> Self {
        Self(ZString::new("/"))
    }
}

#[cfg(feature = "alloc")]
impl Deref for AddressBuf {
    type Target = Address;

    fn deref(&self) -> &Self::Target {
        self.as_address()
    }
}

#[cfg(feature = "alloc")]
impl Borrow<Address> for AddressBuf {
    #[inline]
    fn borrow(&self) -> &Address {
        self.as_address()
    }
}

#[cfg(feature = "alloc")]
impl ToOwned for Address {
    type Owned = AddressBuf;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        AddressBuf(self.0.to_owned())
    }
}

#[cfg(feature = "alloc")]
impl AddressBuf {
    #[inline]
    pub fn new() -> Self {
        Self(ZString::new("/"))
    }

    #[inline]
    pub fn as_address(&self) -> &Address {
        Address::from_zstr_raw(self.0.as_zstr())
    }

    pub fn push(&mut self, zstr: impl AsRef<ZStr>) -> Result<(), Error> {
        let zstr = zstr.as_ref();
        check_charset(zstr.as_bytes())?;
        self.0.push_zstr(zstr);
        Ok(())
    }

    pub fn push_raw(&mut self, zstr: impl AsRef<ZStr>) {
        self.0.push_zstr(zstr.as_ref());
    }

    pub fn push_segment(&mut self, segment: impl AsRef<ZStr>) -> Result<(), Error> {
        let segment = segment.as_ref();
        check_segment(segment.as_bytes())?;
        self.0.push_zstr(segment);
        Ok(())
    }

    pub fn push_segment_raw(&mut self, segment: impl AsRef<ZStr>) {
        self.0.push(SEPARATOR);
        self.0.push_zstr(segment.as_ref());
    }

    #[inline]
    pub fn into_zstring(self) -> ZString {
        self.0
    }

    #[inline]
    pub fn display(&self) -> Display<'_> {
        Display {
            inner: self.0.display(),
        }
    }
}

#[cfg(feature = "alloc")]
impl AsRef<Address> for AddressBuf {
    fn as_ref(&self) -> &Address {
        self
    }
}

#[cfg(feature = "alloc")]
impl AsRef<ZStr> for AddressBuf {
    fn as_ref(&self) -> &ZStr {
        &self.0
    }
}

#[cfg(feature = "alloc")]
impl AsRef<[u8]> for AddressBuf {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl AsRef<Address> for Address {
    fn as_ref(&self) -> &Address {
        self
    }
}

impl AsRef<ZStr> for Address {
    fn as_ref(&self) -> &ZStr {
        &self.0
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<str> for AddressBuf {
    fn eq(&self, other: &str) -> bool {
        self.0.as_bytes() == other.as_bytes()
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<&str> for AddressBuf {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<str> for Address {
    fn eq(&self, other: &str) -> bool {
        self.0.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<&str> for Address {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_bytes() == other.as_bytes()
    }
}

impl Address {
    pub fn new<S: AsRef<ZStr> + ?Sized>(s: &S) -> Result<&Self, Error> {
        Self::from_zstr(s.as_ref())
    }

    #[inline]
    pub const fn from_str(s: &str) -> Result<&Self, Error> {
        Self::from_bytes(s.as_bytes())
    }

    #[inline]
    pub const fn from_str_raw(s: &str) -> &Self {
        Self::from_bytes_raw(s.as_bytes())
    }

    #[inline]
    pub const fn from_bytes(bytes: &[u8]) -> Result<&Self, Error> {
        let zstr = ZStr::from_bytes(bytes);
        Self::from_zstr(zstr)
    }

    #[inline]
    pub const fn from_bytes_raw(bytes: &[u8]) -> &Self {
        let zstr = ZStr::from_bytes(bytes);
        Self::from_zstr_raw(zstr)
    }

    pub const fn from_zstr(zstr: &ZStr) -> Result<&Self, Error> {
        let bytes = zstr.as_bytes();
        match check(bytes) {
            Ok(_) => Ok(Self::from_zstr_raw(zstr)),
            Err(e) => Err(e),
        }
    }

    #[inline]
    pub const fn from_zstr_raw(zstr: &ZStr) -> &Self {
        unsafe { &*(zstr as *const ZStr as *const Address) }
    }

    #[inline]
    pub fn display(&self) -> Display<'_> {
        Display {
            inner: self.0.display(),
        }
    }

    #[inline]
    #[cfg(feature = "alloc")]
    pub fn to_zstring(&self) -> ZString {
        self.0.to_zstring()
    }

    #[cfg(feature = "alloc")]
    pub fn to_address_buf(&self) -> AddressBuf {
        self.to_owned()
    }
}

#[cfg(feature = "alloc")]
impl FromStr for AddressBuf {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AddressBuf::try_from(ZStr::from_bytes(s.as_bytes()))
    }
}

#[cfg(feature = "alloc")]
impl From<&'_ Address> for AddressBuf {
    fn from(address: &'_ Address) -> Self {
        address.to_address_buf()
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<&'_ ZStr> for AddressBuf {
    type Error = Error;

    fn try_from(zstr: &'_ ZStr) -> Result<Self, Self::Error> {
        check(zstr.as_bytes())?;
        return Ok(Self(zstr.to_zstring()));
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<ZString> for AddressBuf {
    type Error = Error;

    fn try_from(zstring: ZString) -> Result<Self, Self::Error> {
        check(zstring.as_bytes())?;
        Ok(Self(zstring))
    }
}

#[cfg(feature = "alloc")]
impl From<&'_ Address> for ZString {
    fn from(address: &'_ Address) -> ZString {
        address.0.to_zstring()
    }
}

#[cfg(feature = "parse")]
impl<'a> Parse<'a> for &'a Address {
    type Error = wire::Error;
    fn parse(parser: &mut Parser<'a>) -> Result<Self, wire::Error> {
        Ok(Address::from_zstr(parser.take_zstr_padded()?)?)
    }
}

#[cfg(feature = "serialize")]
impl_both! {
    impl(Serialize) AddressBuf => Address {
        fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
            w.write(self.0.as_bytes())?;
            w.write_u8(0)?;
            w.write_padding(self.0.len() + 1)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_validation() {
        assert!(Address::new("/").is_ok());
        assert!(Address::new("/a/b/c").is_ok());
        #[cfg(feature = "compat_trailing_slash")]
        assert!(Address::new("/a/b/c/").is_ok());
        #[cfg(feature = "compat_trailing_slash")]
        assert!(Address::new("/a/b/c/").unwrap() == "/a/b/c/");
        #[cfg(not(feature = "compat_trailing_slash"))]
        assert!(Address::new("/a/b/c/").is_err());

        assert!(Address::new("").is_err());
        assert!(Address::new("//").is_err());
        assert!(Address::new("/a//").is_err());
        assert!(Address::new("/a///").is_err());
        assert!(Address::new("/a[]").is_err());
        assert!(Address::new("/{a}").is_err());
        assert!(Address::new("/{a}").is_err());
    }
}

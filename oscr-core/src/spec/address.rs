use super::macros::define_owned_and_ref;
use super::zstr::*;

#[cfg(feature = "parse")]
use super::parser::Parser;
#[cfg(feature = "parse")]
use super::wire::{self, Parse};

#[cfg(feature = "serialize")]
use super::macros::impl_both;
#[cfg(feature = "serialize")]
use super::wire::{Serialize, Write};

#[cfg(feature = "alloc")]
use alloc::borrow::{Borrow, ToOwned};
#[cfg(feature = "alloc")]
use core::ops::Deref;

use crate::Error;

const fn charset_map(charset: &[u8]) -> [bool; 256] {
    let mut table = [false; 256];
    let mut i = 0;
    while i < charset.len() {
        table[charset[i] as usize] = true;
        i += 1;
    }
    table
}

const SEPARATOR: u8 = b'/';
const DISALLOWED: &[u8] = b" #*,?[]{}";
const DISALLOWED_MAP: [bool; 256] = charset_map(DISALLOWED);
const METHOD_DISALLOWED: &[u8] = b" #*,/?[]{}";
const METHOD_DISALLOWED_MAP: [bool; 256] = charset_map(METHOD_DISALLOWED);

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
        Address::new(&self.0)
    }
}

#[cfg(feature = "alloc")]
impl Borrow<Address> for AddressBuf {
    #[inline]
    fn borrow(&self) -> &Address {
        Address::new(&self.0)
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
    pub fn new() -> Self {
        Self(ZString::new("/"))
    }

    pub fn from_zstring(zs: ZString) -> Result<Self, Error> {
        if zs.as_bytes().starts_with(b"/") {
            if let Some(&invalid) = zs.as_bytes().iter().find(|&&b| DISALLOWED_MAP[b as usize]) {
                Err(Error::Address(Some(invalid)))
            } else {
                Ok(Self(zs))
            }
        } else {
            Err(Error::Address(None))
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

impl Address {
    pub fn new<S: AsRef<ZStr> + ?Sized>(s: &S) -> &Self {
        unsafe { &*(s.as_ref() as *const ZStr as *const Address) }
    }
}

#[cfg(feature = "parse")]
impl<'a> Parse<'a> for &'a Address {
    type Error = wire::Error;
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Self::Error> {
        Ok(Address::new(parser.take_zstr_padded()?))
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

define_owned_and_ref! {
    #[
        derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash) =>
        derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)
    ]
    pub struct PatternBuf => Pattern(ZString => ZStr);
}

#[cfg(feature = "alloc")]
impl Default for PatternBuf {
    fn default() -> Self {
        Self(unsafe { ZStr::from_bytes_unchecked(b"/") }.to_zstring())
    }
}

#[cfg(feature = "alloc")]
impl Deref for PatternBuf {
    type Target = Pattern;

    fn deref(&self) -> &Self::Target {
        Pattern::new(&self.0)
    }
}

#[cfg(feature = "alloc")]
impl Borrow<Pattern> for PatternBuf {
    #[inline]
    fn borrow(&self) -> &Pattern {
        Pattern::new(&self.0)
    }
}

#[cfg(feature = "alloc")]
impl ToOwned for Pattern {
    type Owned = PatternBuf;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        PatternBuf(self.0.to_owned())
    }
}

#[cfg(feature = "alloc")]
impl From<&Address> for PatternBuf {
    fn from(value: &Address) -> Self {
        Self(value.0.to_zstring())
    }
}

#[cfg(feature = "alloc")]
impl From<AddressBuf> for PatternBuf {
    fn from(value: AddressBuf) -> Self {
        Self(value.0)
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<str> for PatternBuf {
    fn eq(&self, other: &str) -> bool {
        self.0.as_bytes() == other.as_bytes()
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<&str> for PatternBuf {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<str> for Pattern {
    fn eq(&self, other: &str) -> bool {
        self.0.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<&str> for Pattern {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_bytes() == other.as_bytes()
    }
}

#[cfg(feature = "alloc")]
impl AsRef<Pattern> for PatternBuf {
    fn as_ref(&self) -> &Pattern {
        self
    }
}

#[cfg(feature = "alloc")]
impl AsRef<ZStr> for PatternBuf {
    fn as_ref(&self) -> &ZStr {
        &self.0
    }
}

#[cfg(feature = "alloc")]
impl AsRef<[u8]> for PatternBuf {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl AsRef<Pattern> for Pattern {
    fn as_ref(&self) -> &Pattern {
        self
    }
}

impl AsRef<ZStr> for Pattern {
    fn as_ref(&self) -> &ZStr {
        &self.0
    }
}

impl AsRef<[u8]> for Pattern {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Pattern {
    pub fn new<S: AsRef<ZStr> + ?Sized>(s: &S) -> &Self {
        unsafe { &*(s.as_ref() as *const ZStr as *const Pattern) }
    }
}

#[cfg(feature = "parse")]
impl<'a> Parse<'a> for &'a Pattern {
    type Error = wire::Error;
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Self::Error> {
        Ok(Pattern::new(parser.take_zstr_padded()?))
    }
}

#[cfg(feature = "serialize")]
impl_both! {
    impl(Serialize) PatternBuf => Pattern {
        fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
            w.write(self.0.as_bytes())?;
            w.write_u8(0)?;
            w.write_padding(self.0.len() + 1)?;
            Ok(())
        }
    }
}

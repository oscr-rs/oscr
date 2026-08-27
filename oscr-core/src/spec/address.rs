use super::macros::{define_owned_and_ref, impl_both};
use super::wire::{self, Parse, Serialize, Write};
use super::zstr::{ZStr, ZString};

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
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Address => AddressRef<'a>(ZString => &'a ZStr);
}

impl Default for Address {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Copy for AddressRef<'_> {}

impl Address {
    pub fn from_zstring(zs: ZString) -> Result<Self, Error> {
        if let Some(&invalid) = zs.as_bytes().iter().find(|&&b| DISALLOWED_MAP[b as usize]) {
            Err(Error::InvalidAddress(invalid))
        } else {
            Ok(Self(zs))
        }
    }
}

#[cfg(feature = "parse")]
impl<'a> Parse<'a> for AddressRef<'a> {
    type Error = wire::Error;
    fn parse(parser: &'a mut super::parser::Parser) -> Result<Self, Self::Error> {
        Ok(Self(parser.take_zstr()?))
    }
}

#[cfg(feature = "serialize")]
impl_both! {
    impl(Serialize) Address => AddressRef<'_> {
        fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
            w.write(self.0.as_bytes())?;
            w.write_u8(0)?;
            w.write_padding(self.0.len() + 1)?;
            Ok(())
        }
    }
}

define_owned_and_ref! {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Pattern => PatternRef<'a>(ZString => &'a ZStr);
}

impl Copy for PatternRef<'_> {}

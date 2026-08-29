use super::macros::define_owned_and_ref;
use super::zstr::*;

#[cfg(feature = "alloc")]
use super::address::AddressBuf;
use super::address::{self, Address, InvalidByte, MagicError};

#[cfg(feature = "parse")]
use super::parser::Parser;
#[cfg(feature = "parse")]
use super::wire::{self, Parse};

#[cfg(feature = "serialize")]
use super::macros::impl_both;
#[cfg(feature = "serialize")]
use super::wire::{Serialize, Write};

use core::fmt::{self, Debug};

#[cfg(feature = "alloc")]
use alloc::borrow::{Borrow, ToOwned};
#[cfg(feature = "alloc")]
use core::ops::Deref;

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
impl PatternBuf {
    #[inline]
    fn as_pattern(&self) -> &Pattern {
        Pattern::from_zstr_raw(self.0.as_zstr())
    }
}

#[cfg(feature = "alloc")]
impl Deref for PatternBuf {
    type Target = Pattern;

    fn deref(&self) -> &Self::Target {
        self.as_pattern()
    }
}

#[cfg(feature = "alloc")]
impl Borrow<Pattern> for PatternBuf {
    #[inline]
    fn borrow(&self) -> &Pattern {
        self.as_pattern()
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
        Self(value.to_owned().into_zstring())
    }
}

#[cfg(feature = "alloc")]
impl From<AddressBuf> for PatternBuf {
    fn from(value: AddressBuf) -> Self {
        Self(value.into_zstring())
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

#[derive(Debug, Clone)]
pub enum Error {
    Magic(MagicError),
    Invalid(InvalidByte),
    Slashes(usize),
    Trailing,
    Validation(ValidatorError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Magic(e) => write!(f, "{}", e),
            Self::Invalid(e) => write!(f, "{}", e),
            Self::Slashes(n) => write!(f, "too many slashes ({})", n),
            Self::Trailing => write!(f, "trailing slash"),
            Self::Validation(e) => write!(f, "{}", e),
        }
    }
}

impl core::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct ValidatorError {
    position: usize,
    state: ValidatorState,
    input: Option<u8>,
}

impl fmt::Display for ValidatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(input) = self.input {
            write!(
                f,
                "validation error: position {}, state {:?}, input {}",
                self.position, self.state, input
            )
        } else {
            write!(
                f,
                "validation error: position {}, state {:?}, end",
                self.position, self.state
            )
        }
    }
}

impl core::error::Error for ValidatorError {}

#[derive(Debug)]
pub struct Validator {
    cursor: usize,
    last: u8,
    state: ValidatorState,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidatorState {
    #[default]
    Detached,
    Charset,
    CharsetChar,
    CharsetRange,
    CharsetRangeSus,
    Choice,
}

impl ValidatorState {
    const fn is_accepted(&self) -> bool {
        match self {
            Self::Detached => true,
            _ => false,
        }
    }
}

impl Validator {
    const fn new(cursor: usize, last: u8) -> Self {
        Self {
            cursor,
            last,
            state: ValidatorState::Detached,
        }
    }

    const fn feed_byte(&mut self, byte: u8) -> Result<(), ValidatorError> {
        use ValidatorState::*;

        let new = match (self.state, byte) {
            (Detached, b']' | b'}' | b',') => {
                return Err(self.to_error(Some(byte)));
            }
            (Detached, b'[') => Charset,
            (Charset, b'!') => Charset,
            (Charset | CharsetChar | CharsetRange | CharsetRangeSus, b']') => Detached,
            (Charset | CharsetChar | CharsetRange | CharsetRangeSus, byte)
                if address::is_disallowed(byte) =>
            {
                return Err(self.to_error(Some(byte)));
            }
            (CharsetRangeSus, byte) => {
                return Err(self.to_error(Some(byte)));
            }
            (Charset, _) => CharsetChar,
            (CharsetChar, b'-') => CharsetRange,
            (CharsetChar, _) => CharsetChar,
            (CharsetRange, b'-') => CharsetRangeSus,
            (CharsetRange, _) => CharsetChar,
            (Detached, b'{') => {
                if self.last == b'/' {
                    Choice
                } else {
                    return Err(self.to_error(Some(byte)));
                }
            }
            (Choice, b'}') => Detached,
            (Choice, b',') => Choice,
            (Choice, byte) if address::is_disallowed_segment(byte) => {
                return Err(self.to_error(Some(byte)));
            }
            (Choice, _) => Choice,
            (Detached, byte) if self.last == b'}' => {
                if byte != b'/' {
                    return Err(self.to_error(Some(byte)));
                }
                Detached
            }
            (Detached, _) => Detached,
        };
        self.state = new;
        self.last = byte;
        self.cursor += 1;
        Ok(())
    }

    const fn to_error(&self, input: Option<u8>) -> ValidatorError {
        ValidatorError {
            position: self.cursor,
            state: self.state,
            input,
        }
    }

    const fn validate(&mut self, bytes: &[u8]) -> Result<(), ValidatorError> {
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            if let Err(e) = self.feed_byte(bytes[i]) {
                return Err(e);
            }
            i += 1;
        }

        if self.state.is_accepted() {
            Ok(())
        } else {
            Err(self.to_error(None))
        }
    }
}

#[inline]
const fn is_disallowed(byte: u8) -> bool {
    matches!(byte, b' ' | b'#')
}

#[inline]
const fn is_special(byte: u8) -> bool {
    address::is_disallowed(byte)
}

pub(super) const fn check_pos(bytes: &[u8]) -> Result<Option<usize>, Error> {
    let len = bytes.len();
    let mut i = 0;
    let mut pos = None;
    let mut slashes = 0;

    while i < len {
        let byte = bytes[i];

        if byte == b'/' {
            slashes += 1
        } else {
            slashes = 0;
        }

        if slashes > 2 {
            return Err(Error::Slashes(slashes));
        }

        if is_disallowed(byte) {
            return Err(Error::Invalid(InvalidByte { position: i, byte }));
        }

        if is_special(byte) {
            if pos.is_none() {
                pos = Some(i);
            }
        }
        i += 1;
    }

    Ok(pos)
}

pub(super) const fn check(bytes: &[u8]) -> Result<(), Error> {
    match address::check_magic(bytes) {
        Ok(bytes) => match check_pos(bytes) {
            Ok(None) => Ok(()),
            Ok(Some(pos)) => {
                assert!(pos > 0);
                let last = pos - 1;
                let mut v = Validator::new(pos, bytes[last]);
                match v.validate(bytes) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(Error::Validation(e)),
                }
            }
            Err(e) => Err(e),
        },
        Err(e) => Err(Error::Magic(e)),
    }
}

impl Pattern {
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
        let zstr = ZStr::from_bytes_lossy(bytes);
        Self::from_zstr(zstr)
    }

    #[inline]
    pub const fn from_bytes_raw(bytes: &[u8]) -> &Self {
        let zstr = ZStr::from_bytes_lossy(bytes);
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
        unsafe { &*(zstr as *const ZStr as *const Pattern) }
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    // pub fn segments(&self) -> impl Iterator<Item = SegmentRef<'_>> {
    //     self.0.split(SEPARATOR)
    //         .map(|segment| {
    //         })
    // }

    #[cfg(feature = "alloc")]
    pub fn compile(&self) -> Compiled {
        todo!()
    }

    // pub fn compile_segment_refs<const N: usize>(&self) -> SegmentRefs<'_, N> {}
}

#[cfg(feature = "parse")]
impl<'a> Parse<'a> for &'a Pattern {
    type Error = wire::Error;
    fn parse(parser: &mut Parser<'a>) -> Result<Self, Self::Error> {
        Ok(Pattern::new(parser.take_zstr_padded()?)?)
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

#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
struct Compiled {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
pub struct SegmentRefs<'a, const N: usize> {
    data: [Option<SegmentRef<'a>>; N],
    len: usize,
}

define_owned_and_ref! {
    #[
        derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash) =>
        derive(Debug, Clone)
    ]
    pub enum Segment => SegmentRef<'a> {
        Exact(ZString => &'a ZStr),
        Any,
        AnySegments,
        AnyOf(Vec<ZString> => AnyOfIter<'a>),
        HasPrefix(ZString => &'a ZStr),
        HasSuffix(ZString => &'a ZStr),
        HasAffix((ZString, ZString) => (&'a ZStr, &'a ZStr)),
        Complex(ZString => &'a ZStr),
    }
}

#[derive(Debug, Clone)]
pub struct AnyOfIter<'a> {
    data: &'a ZStr,
}

impl<'a> Iterator for AnyOfIter<'a> {
    type Item = &'a ZStr;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pos) = self.data.as_bytes().iter().position(|&b| b == b',') {
            let (next, rest) = self.data.split_at_checked(pos)?;
            self.data = rest.split_at(1).1;
            Some(next)
        } else if !self.data.is_empty() {
            let data = self.data;
            self.data = ZStr::from_bytes_lossy(&[]);
            Some(data)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_validation() {
        assert!(Pattern::new("/").is_ok());
        assert!(Pattern::new("/a").is_ok());
        #[cfg(feature = "compat_trailing_slash")]
        assert!(Address::new("/a/b/c/").is_ok());
        #[cfg(not(feature = "compat_trailing_slash"))]
        assert!(Address::new("/a/b/c/").is_err());

        assert!(Pattern::new("//a").is_ok());
        assert!(Pattern::new("//a/b").is_ok());
        assert!(Pattern::new("/a//").is_ok());
        assert!(Pattern::new("///a/b").is_err());
        assert!(Pattern::new("/a///").is_err());

        assert!(Pattern::new("").is_err());
        assert!(Pattern::new("a").is_err());
        assert!(Pattern::new("a/").is_err());

        assert!(Pattern::new("/#").is_err());
        assert!(Pattern::new("/ ").is_err());

        assert!(Pattern::new("/*").is_ok());
        assert!(Pattern::new("/?").is_ok());

        assert!(Pattern::new("/a/b/c").is_ok());
        assert!(Pattern::new("/foo/bar").is_ok());

        assert!(Pattern::new("/a/*/c").is_ok());
        #[cfg(feature = "compat_trailing_slash")]
        assert!(Pattern::new("/foo*?/").is_ok());
        #[cfg(not(feature = "compat_trailing_slash"))]
        assert!(Pattern::new("/foo?*/").is_err());

        assert!(Pattern::new("/{foo,bar}").is_ok());
        assert!(Pattern::new("/{foo,bar,baz}").is_ok());
        assert!(Pattern::new("/{foo,}").is_ok());
        assert!(Pattern::new("/foo/{,bar}").is_ok());
        assert!(Pattern::new("/foo/{bar,baz}").is_ok());
        assert!(Pattern::new("/foo/{a,,b}").is_ok());
        assert!(Pattern::new("/foo/{,}").is_ok());
        assert!(Pattern::new("/foo/{*}").is_err());
        assert!(Pattern::new("/foo/{?}").is_err());
        assert!(Pattern::new("/foo/{a,*}").is_err());
        assert!(Pattern::new("/foo/{*,a}").is_err());
        assert!(Pattern::new("/foo/{ }").is_err());
        assert!(Pattern::new("/foo/{[}").is_err());
        assert!(Pattern::new("/foo/{]}").is_err());
        assert!(Pattern::new("/foo{bar}baz").is_err());
        assert!(Pattern::new("/{bar}baz").is_err());
        assert!(Pattern::new("/foo{}").is_err());
        assert!(Pattern::new("/foo{").is_err());
        assert!(Pattern::new("/foo}").is_err());

        assert!(Pattern::new("/foo/bar[a]").is_ok());
        assert!(Pattern::new("/foo/bar[abc]").is_ok());
        assert!(Pattern::new("/foo/bar[a-z]").is_ok());
        assert!(Pattern::new("/foo/bar[z-a]").is_ok());
        assert!(Pattern::new("/foo/bar[-a]").is_ok());
        assert!(Pattern::new("/foo/bar[a-]").is_ok());
        assert!(Pattern::new("/foo/bar[0-9]").is_ok());
        assert!(Pattern::new("/foo/bar[0-9a-f][0-9a-f]").is_ok());
        assert!(Pattern::new("/foo/bar[0-0]").is_ok());
        assert!(Pattern::new("/foo/bar[9-0]").is_ok());
        assert!(Pattern::new("/foo/bar[+--]").is_ok());
        assert!(Pattern::new("/foo/bar[---]").is_ok());
        assert!(Pattern::new("/foo/bar[--b]").is_ok());
        assert!(Pattern::new("/foo/bar[--b]").is_ok());
        assert!(Pattern::new("/foo/bar[!a]").is_ok());
        assert!(Pattern::new("/foo/bar[!abc]").is_ok());
        assert!(Pattern::new("/foo/bar[!0-9]").is_ok());
        assert!(Pattern::new("/foo/bar[a!b]").is_ok());
        assert!(Pattern::new("/foo/bar[!--a]").is_ok());
        assert!(Pattern::new("/foo/bar[!---]").is_ok());
        assert!(Pattern::new("/foo/bar[a--b]").is_err());
        assert!(Pattern::new("/foo/bar[---b]").is_err());
        assert!(Pattern::new("/foo/bar[a--b]").is_err());
        assert!(Pattern::new("/foo/bar[ ]").is_err());
        assert!(Pattern::new("/foo/bar[#]").is_err());
        assert!(Pattern::new("/foo/bar[*]").is_err());
        assert!(Pattern::new("/foo/bar[?]").is_err());
        assert!(Pattern::new("/foo/bar[! ]").is_err());
        assert!(Pattern::new("/foo/bar[!#]").is_err());
        assert!(Pattern::new("/foo/bar[!*]").is_err());
        assert!(Pattern::new("/foo/bar[!?]").is_err());
    }
}

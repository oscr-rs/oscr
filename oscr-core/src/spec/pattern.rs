use super::address::Error;
use super::macros::define_owned_and_ref;
use super::zstr::*;

#[cfg(feature = "alloc")]
use super::address::AddressBuf;
use super::address::{self, Address, InvalidByte};

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
use alloc::vec::Vec;
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
                if address::is_byte_disallowed(byte) =>
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
            (Choice, byte) if address::is_byte_disallowed_segment(byte) => {
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
const fn is_byte_disallowed(byte: u8) -> bool {
    matches!(byte, b' ' | b'#')
}

#[inline]
const fn is_byte_special(byte: u8) -> bool {
    address::is_byte_disallowed(byte)
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

        if is_byte_disallowed(byte) {
            return Err(Error::Invalid(InvalidByte { position: i, byte }));
        }

        if is_byte_special(byte) {
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
        unsafe { &*(zstr as *const ZStr as *const Pattern) }
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn segment_refs(&self) -> SegmentRefIter<'_> {
        SegmentRefIter {
            bytes: self.as_bytes(),
        }
    }

    #[cfg(feature = "alloc")]
    pub fn segments(&self) -> Vec<Segment> {
        self.segment_refs()
            .map(|segment| segment.to_owned())
            .collect()
    }

    #[cfg(feature = "alloc")]
    pub fn compile(&self) -> Compiled {
        todo!()
    }

    #[inline]
    #[cfg(feature = "alloc")]
    pub fn to_zstring(&self) -> ZString {
        self.0.to_zstring()
    }
}

#[cfg(feature = "alloc")]
impl From<&'_ Pattern> for ZString {
    fn from(pattern: &'_ Pattern) -> ZString {
        pattern.0.to_zstring()
    }
}

pub struct SegmentRefIter<'a> {
    bytes: &'a [u8],
}

impl<'a> SegmentRefIter<'a> {
    fn current(&self) -> Option<u8> {
        self.bytes.first().copied()
    }

    fn take_until_slash_or_eof(&mut self) -> &'a [u8] {
        let pos = self
            .bytes
            .iter()
            .position(|&b| b == b'/')
            .unwrap_or(self.bytes.len());
        let taken = &self.bytes[..pos];
        self.bytes = &self.bytes[pos..];
        taken
    }

    fn take_slashes(&mut self) -> usize {
        let non_slash = self
            .bytes
            .iter()
            .position(|&b| b != b'/')
            .unwrap_or(self.bytes.len());
        self.bytes = &self.bytes[non_slash..];
        non_slash
    }
}

impl<'a> Iterator for SegmentRefIter<'a> {
    type Item = SegmentRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current()?;

        if current == b'/' {
            let slashes = self.take_slashes();
            match slashes {
                1 => {
                    let segment = self.take_until_slash_or_eof();
                    SegmentRef::parse(unsafe { ZStr::from_bytes_unchecked(segment) })
                }
                2 => Some(SegmentRef::AnySegments),
                _ => {
                    self.bytes = &[];
                    None
                }
            }
        } else {
            let segment = self.take_until_slash_or_eof();
            SegmentRef::parse(unsafe { ZStr::from_bytes_unchecked(segment) })
        }
    }
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
pub struct Compiled {
    segments: Vec<CompiledSegment>,
}

impl Compiled {
    pub fn segments(&self) -> &[CompiledSegment] {
        &self.segments
    }
}

#[derive(Debug, Clone)]
pub struct CompiledSegment;

pub type CharSetLookup = [bool; 256];
#[allow(dead_code)]
pub type CharSetLookupCompact = [u8; 32];

#[derive(Debug, Clone)]
pub struct SegmentRefs<'a, const N: usize> {
    _data: [Option<SegmentRef<'a>>; N],
    _len: usize,
}

define_owned_and_ref! {
    #[
        derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash) =>
        derive(Debug, Clone, PartialEq, Eq)
    ]
    pub enum Segment => SegmentRef<'a> {
        Exact(ZString => &'a ZStr),
        Any,
        AnySegments,
        AnyOf(Vec<ZString> => AnyOfIter<'a>),
        HasPrefix(ZString => &'a ZStr),
        HasSuffix(ZString => &'a ZStr),
        HasAffix((ZString, ZString) => (&'a ZStr, &'a ZStr)),
        Pat(ZString => &'a ZStr),
    }
}

impl SegmentRef<'_> {
    #[cfg(feature = "alloc")]
    fn to_owned(&self) -> Segment {
        match self {
            Self::Exact(exact) => Segment::Exact(exact.to_zstring()),
            Self::Any => Segment::Any,
            Self::AnySegments => Segment::AnySegments,
            Self::AnyOf(choices) => {
                Segment::AnyOf(choices.clone().map(|c| c.to_zstring()).collect())
            }
            Self::HasPrefix(prefix) => Segment::HasPrefix(prefix.to_zstring()),
            Self::HasSuffix(suffix) => Segment::HasSuffix(suffix.to_zstring()),
            Self::HasAffix((prefix, suffix)) => {
                Segment::HasAffix((prefix.to_zstring(), suffix.to_zstring()))
            }
            Self::Pat(pat) => Segment::Pat(pat.to_zstring()),
        }
    }
}

impl<'a> SegmentRef<'a> {
    const fn find_special_once(zstr: &'a ZStr) -> Result<Option<(u8, usize)>, ()> {
        let bytes = zstr.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        let mut found = None;
        while i < len {
            if is_byte_special(bytes[i]) {
                if let Some(_twice) = found.replace((bytes[i], i)) {
                    return Err(());
                }
            }
            i += 1;
        }
        Ok(found)
    }

    pub fn parse(zstr: &'a ZStr) -> Option<Self> {
        let (first, rest) = zstr.split_first()?;
        match *first {
            b'{' => {
                let choices = rest.strip_suffix(b"}")?;
                return Some(Self::AnyOf(AnyOfIter { data: choices }));
            }
            b'*' if rest.is_empty() => {
                return Some(Self::Any);
            }
            _ => {}
        }

        match Self::find_special_once(zstr) {
            Ok(Some((b'*', pos))) => {
                if pos == 0 {
                    // pattern "*suffix"
                    let (_, after) = zstr.split_at(pos + 1);
                    return Some(Self::HasSuffix(after));
                } else if pos == zstr.len() - 1 {
                    // pattern "prefix*"
                    let (before, _) = zstr.split_at(pos);
                    return Some(Self::HasPrefix(before));
                } else {
                    // pattern "prefix*suffix"
                    let (before, star_and_after) = zstr.split_at(pos);
                    let (_star, after) = star_and_after.split_at(1);
                    return Some(Self::HasAffix((before, after)));
                }
            }
            Ok(None) => return Some(Self::Exact(zstr)),
            Ok(Some(_)) | Err(_) => return Some(Self::Pat(zstr)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            self.data = ZStr::from_bytes(&[]);
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
        assert!(Pattern::new("/a/b/c/").is_ok());
        #[cfg(not(feature = "compat_trailing_slash"))]
        assert!(Pattern::new("/a/b/c/").is_err());

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

    macro_rules! assert_segments_iter {
        ($iter:expr, $expected:expr) => {{
            let mut iter = $iter;
            let expected = $expected;
            for (idx, exp) in expected.iter().enumerate() {
                assert_eq!(
                    iter.next()
                        .unwrap_or_else(|| panic!("iterator ended early at index {}", idx)),
                    *exp,
                    "mismatch at index {}",
                    idx,
                );
            }
            assert!(
                iter.next().is_none(),
                "iterator has remaining elements after expected end"
            );
        }};
    }

    #[test]
    fn pattern_segments() {
        let pattern = Pattern::new("/{foo,bar}").unwrap();
        assert_segments_iter!(
            pattern.segment_refs(),
            [SegmentRef::AnyOf(AnyOfIter {
                data: "foo,bar".into()
            })]
        );
        let pattern = Pattern::new("//a/b").unwrap();
        assert_segments_iter!(
            pattern.segment_refs(),
            [
                SegmentRef::AnySegments,
                SegmentRef::Exact("a".into()),
                SegmentRef::Exact("b".into())
            ]
        );
        let pattern = Pattern::new("/abc*/*def/foo*bar").unwrap();
        assert_segments_iter!(
            pattern.segment_refs(),
            [
                SegmentRef::HasPrefix("abc".into()),
                SegmentRef::HasSuffix("def".into()),
                SegmentRef::HasAffix(("foo".into(), "bar".into()))
            ]
        );
        let pattern = Pattern::new("/abc*/*def/foo*bar").unwrap();
        assert_segments_iter!(
            pattern.segment_refs(),
            [
                SegmentRef::HasPrefix("abc".into()),
                SegmentRef::HasSuffix("def".into()),
                SegmentRef::HasAffix(("foo".into(), "bar".into()))
            ]
        );
    }
}

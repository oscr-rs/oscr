use super::address::{Address, Pattern};
use super::arg::ArgRef;
use super::arg::Tag;
use super::macros::{define_owned_and_ref, impl_both};
use super::time::TimeTag;

#[cfg(feature = "alloc")]
use super::address::PatternBuf;
#[cfg(feature = "alloc")]
use super::arg::Arg;

#[cfg(feature = "parse")]
use super::parser::Parser;
#[cfg(feature = "parse")]
use super::wire::{self, Parse};

#[cfg(feature = "serialize")]
use super::wire::{Serialize, Write};

#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[derive(Debug, Default, Clone)]
pub struct ArgsIter<'a> {
    tags: Option<&'a [Tag]>,
    data: &'a [u8],
}

impl<'a> ArgsIter<'a> {
    pub fn is_tagged(&self) -> bool {
        self.tags.is_some()
    }

    pub fn tags(&self) -> Option<&[Tag]> {
        self.tags
    }

    pub fn data(&self) -> &[u8] {
        self.data
    }

    pub fn to_coerced(&self, tags: &'a [Tag]) -> Self {
        Self {
            tags: Some(tags),
            data: self.data,
        }
    }

    #[cfg(feature = "parse")]
    pub fn take_coerced(&mut self, tag: Tag) -> Result<ArgRef<'a>, wire::Error> {
        self.tags = None;
        let mut parser = Parser::new(self.data);
        let arg = ArgRef::parse_tag(&mut parser, tag)?;
        Ok(arg)
    }
}

#[cfg(feature = "parse")]
impl<'a> Iterator for ArgsIter<'a> {
    type Item = Result<ArgRef<'a>, wire::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let (tag, rest) = self.tags.as_ref()?.split_first()?;
        let mut parser = Parser::new(self.data);
        let result = ArgRef::parse_tag(&mut parser, *tag);
        let _ = self.tags.insert(rest);
        self.data = parser.remaining();
        Some(result)
    }
}

define_owned_and_ref! {
    #[derive(Debug, Default, Clone, PartialEq) => derive(Debug, Clone)]
    pub struct Message => MessageRef<'a> {
        pattern: PatternBuf => &'a Pattern,
        args: Vec<Arg> => ArgsIter<'a>,
    }
}

#[cfg(feature = "alloc")]
impl Message {
    pub fn builder() -> MessageBuilder {
        MessageBuilder::default()
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Default, Clone)]
pub struct MessageBuilder(Message);

#[cfg(feature = "alloc")]
impl MessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn address(&mut self, address: impl AsRef<Address>) -> &mut Self {
        self.0.pattern = address.as_ref().into();
        self
    }

    pub fn pattern(&mut self, pattern: impl AsRef<Pattern>) -> &mut Self {
        self.0.pattern = pattern.as_ref().to_owned();
        self
    }

    pub fn arg(&mut self, arg: Arg) -> &mut Self {
        self.0.args.push(arg);
        self
    }

    pub fn build(&mut self) -> Message {
        core::mem::take(&mut self.0)
    }
}

#[cfg(feature = "alloc")]
impl Message {
    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn args(&self) -> &[Arg] {
        &self.args
    }
}

impl MessageRef<'_> {
    pub fn pattern(&self) -> &Pattern {
        self.pattern
    }

    pub fn args(&self) -> ArgsIter<'_> {
        self.args.clone()
    }

    #[cfg(all(feature = "alloc", feature = "parse"))]
    pub fn to_owned(&self) -> Result<Message, wire::Error> {
        Ok(Message {
            pattern: self.pattern.to_owned(),
            args: self
                .args
                .clone()
                .map(|result| result.map(|arg| arg.to_owned()))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[cfg(feature = "parse")]
impl<'a> Parse<'a> for MessageRef<'a> {
    type Error = wire::Error;

    fn parse(parser: &mut Parser<'a>) -> Result<Self, Self::Error> {
        use core::slice::from_raw_parts;
        let pattern = <&Pattern>::parse(parser)?;
        let peeked = parser.peek().ok();

        if peeked == Some(b',') {
            let tag_str = parser.take_zstr_padded()?.strip_prefix(b',').unwrap();

            if let Some(&invalid) = tag_str
                .as_bytes()
                .iter()
                .find(|&&b| Tag::try_from(b).is_err())
            {
                Err(wire::Error::Tag(invalid))
            } else {
                let tags = unsafe {
                    from_raw_parts(tag_str.as_bytes().as_ptr() as *const Tag, tag_str.len())
                };
                Ok(Self {
                    pattern,
                    args: ArgsIter {
                        tags: Some(tags),
                        data: parser.remaining(),
                    },
                })
            }
        } else {
            #[cfg(not(feature = "compat_optional_tag_string"))]
            {
                Err(wire::Error::TagString)
            }
            #[cfg(feature = "compat_optional_tag_string")]
            {
                Ok(Self {
                    pattern,
                    args: ArgsIter {
                        tags: None,
                        data: parser.remaining(),
                    },
                })
            }
        }
    }
}

#[cfg(all(feature = "serialize", feature = "alloc"))]
impl Serialize for Message {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
        self.pattern.serialize(w)?;
        w.write_u8(b',')?;
        for arg in self.args.iter() {
            w.write_u8(arg.tag().as_byte())?;
        }
        w.write_u8(0)?;
        w.write_padding(self.args.len() + 2)?;
        for arg in self.args.iter() {
            arg.serialize(w)?;
        }
        Ok(())
    }
}

#[cfg(feature = "serialize")]
impl Serialize for MessageRef<'_> {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
        self.pattern.serialize(w)?;
        if let Some(tags) = self.args.tags() {
            w.write_u8(b',')?;
            for tag in tags {
                w.write_u8(tag.as_byte())?;
            }
            w.write_u8(0)?;
            w.write_padding(tags.len() + 2)?;
        }
        w.write(self.args.data())?;
        Ok(())
    }
}

define_owned_and_ref! {
    #[derive(Debug, Clone, PartialEq) => derive(Debug, Clone)]
    pub enum Packet => PacketRef<'a> {
        Message(Message => MessageRef<'a>),
        Bundle(Bundle => BundleRef<'a>),
    }
}

impl PacketRef<'_> {
    #[cfg(all(feature = "alloc", feature = "parse"))]
    pub fn to_owned(&self) -> Result<Packet, wire::Error> {
        match self {
            Self::Message(message) => Ok(Packet::Message(message.to_owned()?)),
            Self::Bundle(bundle) => Ok(Packet::Bundle(bundle.to_owned()?)),
        }
    }
}

#[cfg(feature = "parse")]
impl<'a> Parse<'a> for PacketRef<'a> {
    type Error = wire::Error;

    fn parse(parser: &mut Parser<'a>) -> Result<Self, Self::Error> {
        match parser.peek().ok() {
            Some(b'/') => Ok(Self::Message(MessageRef::parse(parser)?)),
            Some(b'#') => Ok(Self::Bundle(BundleRef::parse(parser)?)),
            other => Err(wire::Error::Packet(other)),
        }
    }
}

#[cfg(feature = "serialize")]
impl_both! {
    impl(Serialize) Packet => PacketRef<'_> {
        fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
            match self {
                Self::Message(message) => {
                    message.serialize(w)
                }
                Self::Bundle(bundle) => {
                    bundle.serialize(w)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PacketsIter<'a> {
    data: &'a [u8],
}

impl<'a> PacketsIter<'a> {
    pub fn data(&self) -> &'a [u8] {
        self.data
    }
}

#[cfg(feature = "parse")]
impl<'a> Iterator for PacketsIter<'a> {
    type Item = Result<PacketRef<'a>, wire::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut parser = Parser::new(self.data);
        if parser.eof() {
            None
        } else {
            match parser.take_be_i32() {
                Ok(len) => {
                    assert!(len % 4 == 0);
                    Some(PacketRef::parse(&mut parser))
                }
                Err(e) => Some(Err(e.into())),
            }
        }
    }
}

define_owned_and_ref! {
    #[derive(Debug, Default, Clone, PartialEq) => derive(Debug, Clone)]
    pub struct Bundle => BundleRef<'a> {
        time: TimeTag => TimeTag,
        elements: Vec<Packet> => PacketsIter<'a>,
    }
}

#[cfg(feature = "alloc")]
impl Bundle {
    pub fn builder() -> BundleBuilder {
        BundleBuilder::default()
    }

    pub fn elements(&self) -> &[Packet] {
        &self.elements
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Default)]
pub struct BundleBuilder(Bundle);

#[cfg(feature = "alloc")]
impl BundleBuilder {
    pub fn time(&mut self, time: TimeTag) -> &mut Self {
        self.0.time = time;
        self
    }

    pub fn message(&mut self, message: Message) -> &mut Self {
        self.0.elements.push(Packet::Message(message));
        self
    }

    pub fn bundle(&mut self, bundle: Bundle) -> &mut Self {
        self.0.elements.push(Packet::Bundle(bundle));
        self
    }

    pub fn build(&mut self) -> Bundle {
        core::mem::take(&mut self.0)
    }
}

impl BundleRef<'_> {
    pub fn elements(&self) -> PacketsIter<'_> {
        self.elements.clone()
    }

    #[cfg(all(feature = "alloc", feature = "parse"))]
    pub fn to_owned(&self) -> Result<Bundle, wire::Error> {
        Ok(Bundle {
            time: self.time,
            elements: self
                .elements
                .clone()
                .map(|result| result.and_then(|e| e.to_owned()))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl_both! {
    impl Bundle => BundleRef<'_> {
        pub fn time(&self) -> TimeTag {
            self.time
        }
    }
}

#[cfg(feature = "parse")]
impl<'a> Parse<'a> for BundleRef<'a> {
    type Error = wire::Error;

    fn parse(parser: &mut Parser<'a>) -> Result<Self, Self::Error> {
        let time = parser.take_be_u64().map(TimeTag::from_raw)?;
        Ok(Self {
            time,
            elements: PacketsIter {
                data: parser.remaining(),
            },
        })
    }
}

#[cfg(all(feature = "serialize", feature = "alloc"))]
impl Serialize for Bundle {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
        w.write(b"#bundle\x00")?;
        self.time.serialize(w)?;
        for elem in self.elements.iter() {
            w.write_be_u32(elem.len() as _)?;
            elem.serialize(w)?;
        }
        Ok(())
    }
}

#[cfg(all(feature = "serialize"))]
impl Serialize for BundleRef<'_> {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
        w.write(b"#bundle\x00")?;
        self.time.serialize(w)?;
        w.write(self.elements.data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "parse")]
    fn parse_bytes<'a, T: Parse<'a>>(bytes: &'a [u8]) -> Result<T, T::Error> {
        T::parse(&mut Parser::new(bytes))
    }

    // https://opensoundcontrol.stanford.edu/spec-1_0-examples.html
    #[cfg(all(feature = "parse", feature = "alloc"))]
    #[test]
    fn spec_1_0_examples() {
        use crate::{ZStr, ZString};

        let oscillator =
            parse_bytes::<MessageRef>(b"/oscillator/4/frequency\x00,f\x00\x00\x43\xdc\x00\x00")
                .unwrap();
        assert_eq!(oscillator.pattern(), "/oscillator/4/frequency");
        assert_eq!(
            oscillator.args().collect::<Result<Vec<_>, _>>().unwrap(),
            [ArgRef::Float(440.0f32)],
        );
        let owned = oscillator.to_owned().unwrap();
        assert_eq!(
            owned,
            Message {
                pattern: Pattern::new("/oscillator/4/frequency").to_owned(),
                args: vec![Arg::Float(440.0f32)]
            }
        );

        let foo = parse_bytes::<MessageRef>(
            b"/foo\x00\x00\x00\x00,iisff\x00\x00\x00\x00\x03\xe8\xff\xff\xff\xff\x68\x65\x6c\x6c\x6f\x00\x00\x00\x3f\x9d\xf3\xb6\x40\xb5\xb2\x2d",
        ).unwrap();
        assert_eq!(foo.pattern(), "/foo");
        assert_eq!(
            foo.args().collect::<Result<Vec<_>, _>>().unwrap(),
            [
                ArgRef::Int32(1000),
                ArgRef::Int32(-1),
                ArgRef::String(ZStr::new("hello")),
                ArgRef::Float(1.234f32),
                ArgRef::Float(5.678f32),
            ]
        );
        let owned = foo.to_owned().unwrap();
        assert_eq!(
            owned,
            Message {
                pattern: Pattern::new("/foo").to_owned(),
                args: vec![
                    Arg::Int32(1000),
                    Arg::Int32(-1),
                    Arg::String(ZString::from("hello")),
                    Arg::Float(1.234f32),
                    Arg::Float(5.678f32),
                ]
            }
        );
    }
}

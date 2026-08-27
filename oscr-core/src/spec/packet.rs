use super::address::{Address, AddressRef};
use super::arg::Tag;
use super::arg::{Arg, ArgRef};
use super::macros::{define_owned_and_ref, impl_both};
use super::parser::Parser;
use super::time::TimeTag;
use super::wire::{self, Parse, Serialize, Write};

#[derive(Debug, Default, Clone)]
pub struct ArgsLazy<'a> {
    tags: Option<&'a [Tag]>,
    data: &'a [u8],
}

impl<'a> ArgsLazy<'a> {
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

    pub fn take_coerced(&mut self, tag: Tag) -> Result<ArgRef<'a>, wire::Error> {
        self.tags = None;
        let mut parser = Parser::new(self.data);
        let arg = ArgRef::parse_tag(&mut parser, tag)?;
        Ok(arg)
    }
}

#[cfg(feature = "alloc")]
type ArgsRef<'a> = Vec<ArgRef<'a>>;
#[cfg(not(feature = "alloc"))]
type ArgsRef<'a> = ArgsLazy<'a>;

impl<'a> Iterator for ArgsLazy<'a> {
    type Item = Result<ArgRef<'a>, wire::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let (tag, rest) = self.tags.as_ref()?.split_first()?;
        let mut parser = Parser::new(self.data);
        let result = ArgRef::parse_tag(&mut parser, *tag);
        let _ = self.tags.insert(rest);
        Some(result)
    }
}

define_owned_and_ref! {
    #[derive(Debug, Clone)]
    pub struct Message => MessageRef<'a> {
        address: Address => AddressRef<'a>,
        args: Vec<Arg> => ArgsRef<'a>,
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            address: Address::default(),
            args: Vec::default(),
        }
    }
}

impl Message {
    pub fn builder() -> MessageBuilder {
        MessageBuilder::default()
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Default, Clone)]
pub struct MessageBuilder(Message);

impl MessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn address(&mut self, address: Address) -> &mut Self {
        self.0.address = address;
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

#[cfg(feature = "parse")]
// impl<'a> Parse<'a> for MessageRef<'a> {
//     type Error = wire::Error;

//     fn parse(parser: &'a mut Parser) -> Result<Self, Self::Error> {
//         use core::slice::from_raw_parts;
//         let address = AddressRef::parse(parser)?;
//         let peeked = parser.peek().ok();

//         if peeked == Some(b',') {
//             parser.advance(1).unwrap();
//             let tag_str = parser.take_zstr()?;
//             let tags = unsafe { from_raw_parts(tag_str.as_ptr() as *const Tag, tag_str.len()) };
//         } else {
//             #[cfg(not(feature = "compat_optional_tag_string"))]
//             { Err(wire::Error::MissingTagString) }
//             #[cfg(feature = "compat_optional_tag_string")]
//             {  }
//         }
//     }
// }
#[cfg(feature = "alloc")]
impl_both! {
    impl(Serialize) Message => MessageRef<'_> {
        fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
            self.address.serialize(w)?;
            w.write_u8(b',')?;
            for arg in self.args.iter() {
                w.write_u8(arg.tag().as_byte())?;
            }
            w.write_u8(0)?;
            w.write_padding(self.args.len() + 2 )?;
            for arg in self.args.iter() {
                arg.serialize(w)?;
            }
            Ok(())
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl Serialize for MessageRef<'_> {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
        self.address.serialize(w)?;
        if let Some(tags) = self.args {
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
    #[derive(Debug, Clone)]
    pub enum Packet => PacketRef<'a> {
        Message(Message => MessageRef<'a>),
        Bundle(Bundle => BundleRef<'a>),
    }
}

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

define_owned_and_ref! {
    #[derive(Debug, Default, Clone)]
    pub struct Bundle => BundleRef<'a> {
        time: TimeTag => TimeTag,
        elements: Vec<Packet> => &'a [PacketRef<'a>],
    }
}

impl Copy for BundleRef<'_> {}

impl Bundle {
    pub fn builder() -> BundleBuilder {
        BundleBuilder::default()
    }

    pub fn elements(&self) -> &[Packet] {
        &self.elements
    }
}

#[derive(Debug, Clone, Default)]
pub struct BundleBuilder(Bundle);

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

impl_both! {
    impl Bundle => BundleRef<'_> {
        pub fn time(&self) -> TimeTag {
            self.time
        }
    }
}

#[cfg(feature = "serialize")]
impl_both! {
    impl(Serialize) Bundle => BundleRef<'_> {
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
}

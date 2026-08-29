/// Defines an enum with a lookup table to construct variant from raw value.
macro_rules! define_tags {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $variant:ident = $byte:literal ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $($variant = $byte),*
        }

        #[cfg(feature = "lut_tag")]
        impl $name {
            pub(crate) const LOOKUP: [Option<Self>; 256] = {
                let mut table = [None; 256];
                $(table[$byte as usize] = Some(Self::$variant);)*
                table
            };

            pub fn from_byte(byte: u8) -> Option<Self> {
                Self::LOOKUP[byte as usize]
            }
        }

        #[cfg(not(feature = "lut_tag"))]
        impl $name {
            pub fn from_byte(byte: u8) -> Option<Self> {
                match byte {
                    $($byte => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

/// Defines owned and ref version of the same struct or enum.
///
/// # Examples
///
/// ```ignore
/// define_owned_and_ref! {
///     struct Data => DataRef<'a>([u8; 512] => &[u8]);
/// }
/// ```
macro_rules! define_owned_and_ref {
    (
        $(#[$meta:meta])*
        $vis:vis struct $owned:ident => $ref:ident $(<$lifetime:lifetime>)? {
            $(
                $field:ident : $ty:ty => $ref_ty:ty
            ),* $(,)?
        }
    ) => {
        #[cfg(feature = "alloc")]
        $(#[$meta])*
        $vis struct $owned { $($field: $ty),* }

        $(#[$meta])*
        $vis struct $ref$(<$lifetime>)? { $($field: $ref_ty),* }
    };
    (
        $(#[$meta:meta => $ref_meta:meta])*
        $vis:vis struct $owned:ident => $ref:ident $(<$lifetime:lifetime>)? {
            $(
                $field:ident : $ty:ty => $ref_ty:ty
            ),* $(,)?
        }
    ) => {
        #[cfg(feature = "alloc")]
        $(#[$meta])*
        $vis struct $owned { $($field: $ty),* }

        $(#[$ref_meta])*
        $vis struct $ref$(<$lifetime>)? { $($field: $ref_ty),* }
    };
    (
        $(#[$meta:meta])*
        $vis:vis struct $owned:ident => $ref:ident $(<$lifetime:lifetime>)? (
            $(
                $ty:ty => $ref_ty:ty
            ),* $(,)?
        ) $(;)?
    ) => {
        #[cfg(feature = "alloc")]
        $(#[$meta])*
        $vis struct $owned ( $($ty),* );

        $(#[$meta])*
        $vis struct $ref$(<$lifetime>)? ( $($ref_ty),* );
    };
    (
        $(#[$meta:meta => $ref_meta:meta])*
        $vis:vis struct $owned:ident => $ref:ident $(<$lifetime:lifetime>)? (
            $(
                $ty:ty => $ref_ty:ty
            ),* $(,)?
        ) $(;)?
    ) => {
        #[cfg(feature = "alloc")]
        $(#[$meta])*
        $vis struct $owned ( $($ty),* );

        $(#[$ref_meta])*
        $vis struct $ref$(<$lifetime>)? ( $($ref_ty),* );
    };
    (
        $(#[$meta:meta])*
        $vis:vis enum $owned:ident => $ref:ident $(<$lifetime:lifetime>)? {
            $(
                $variant:ident$(($ty:ty => $ref_ty:ty))?
            ),* $(,)?
        }
    ) => {
        #[cfg(feature = "alloc")]
        $(#[$meta])*
        $vis enum $owned { $($variant$(($ty))?),* }

        $(#[$meta])*
        $vis enum $ref$(<$lifetime>)? { $($variant$(($ref_ty))?),* }
    };
    (
        $(#[$meta:meta => $ref_meta:meta])*
        $vis:vis enum $owned:ident => $ref:ident $(<$lifetime:lifetime>)? {
            $(
                $variant:ident$(($ty:ty => $ref_ty:ty))?
            ),* $(,)?
        }
    ) => {
        #[cfg(feature = "alloc")]
        $(#[$meta])*
        $vis enum $owned { $($variant$(($ty))?),* }

        $(#[$ref_meta])*
        $vis enum $ref$(<$lifetime>)? { $($variant$(($ref_ty))?),* }
    };
}

macro_rules! impl_both {
    (impl $a:ident => $b:ident $(<$lifetime:lifetime>)? { $($tt:tt)* }) => {
        #[cfg(feature = "alloc")]
        impl $a { $($tt)* }
        impl $b $(<$lifetime>)? { $($tt)* }
    };
    (impl($trait:ty) $a:ident => $b:ident $(<$lifetime:lifetime>)? { $($tt:tt)* }) => {
        #[cfg(feature = "alloc")]
        impl $trait for $a { $($tt)* }
        impl $trait for $b $(<$lifetime>)? { $($tt)* }
    };
}

pub(super) use define_owned_and_ref;
pub(super) use define_tags;
pub(super) use impl_both;

#[cfg(test)]
mod tests {
    #[test]
    fn define_tags_simple() {
        define_tags! {
            #[repr(u8)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum SimpleTag {
                Foo = b'1',
                Bar = b'2',
            }
        }

        assert_eq!(SimpleTag::from_byte(b'1'), Some(SimpleTag::Foo));
        assert_eq!(SimpleTag::from_byte(b'2'), Some(SimpleTag::Bar));
        assert_eq!(SimpleTag::from_byte(b'3'), None);
    }

    #[test]
    fn define_owned_and_ref_simple() {
        #[cfg(feature = "alloc")]
        use alloc::string::String;

        define_owned_and_ref! {
            #[derive(Debug)]
            struct Simple => SimpleRef<'a>(String => &'a str);
        }

        #[cfg(feature = "alloc")]
        assert_eq!(Simple("a".into()).0, "a");

        assert_eq!(SimpleRef("a").0, "a");
    }

    #[test]
    fn define_owned_and_ref_meta() {
        #[cfg(feature = "alloc")]
        use alloc::string::String;

        define_owned_and_ref! {
            #[derive(Debug, Clone, PartialEq, Eq) => derive(Debug, Clone, Copy, PartialEq, Eq)]
            struct Simple => SimpleRef<'a>(String => &'a str);
        }

        #[cfg(feature = "alloc")]
        impl PartialEq<SimpleRef<'_>> for Simple {
            fn eq(&self, other: &SimpleRef<'_>) -> bool {
                self.0 == other.0
            }
        }

        let simple_ref = SimpleRef("a");
        let copied = simple_ref;
        // we can still access `simple_ref` so it was copied rather than moved.
        assert_eq!(simple_ref.0, "a");
        assert_eq!(copied.0, "a");

        #[cfg(feature = "alloc")]
        assert_eq!(Simple("a".into()), simple_ref);
    }

    #[test]
    fn impl_both_simple() {
        #[cfg(feature = "alloc")]
        use alloc::string::String;

        define_owned_and_ref! {
            struct Simple => SimpleRef<'a>(String => &'a str);
        }

        impl_both!(
            impl Simple => SimpleRef<'_> {
                fn to_view(&self) -> &[u8] {
                    self.0.as_bytes()
                }
            }
        );

        #[cfg(feature = "alloc")]
        assert_eq!(Simple("abc".into()).to_view(), b"abc");

        assert_eq!(SimpleRef("abc").to_view(), b"abc");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn impl_both_trait() {
        #[cfg(feature = "alloc")]
        use alloc::string::String;

        trait ToBytes {
            fn to_bytes(&self) -> &[u8];
        }

        impl_both!(
            impl(ToBytes) String => str {
                fn to_bytes(&self) -> &[u8] {
                    self.as_bytes()
                }
            }
        );

        #[cfg(feature = "alloc")]
        assert_eq!(String::to_bytes(&String::from("abc")), b"abc");

        assert_eq!(str::to_bytes("abc"), b"abc");
    }
}

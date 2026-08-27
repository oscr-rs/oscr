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
}

macro_rules! impl_both {
    (impl $a:ident => $b:ident $(<$lifetime:lifetime>)? { $($tt:tt)* }) => {
        impl $a { $($tt)* }
        impl $b $(<$lifetime>)? { $($tt)* }
    };
    (impl($trait:ty) $a:ident => $b:ident $(<$lifetime:lifetime>)? { $($tt:tt)* }) => {
        impl $trait for $a { $($tt)* }
        impl $trait for $b $(<$lifetime>)? { $($tt)* }
    };
}

pub(super) use define_owned_and_ref;
pub(super) use define_tags;
pub(super) use impl_both;

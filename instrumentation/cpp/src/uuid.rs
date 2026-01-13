use std::fmt::Display;

use uuid::Uuid;

#[cxx::bridge(namespace = "uuid")]
pub mod ffi {

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Ord, PartialOrd)]
    pub struct UUID {
        pub high_bits: u64,
        pub low_bits: u64,
    }

    extern "Rust" {
        #[Self = "UUID"]
        fn now_v7() -> UUID;

        fn to_string(self: &UUID) -> String;
    }
}

impl From<ffi::UUID> for Uuid {
    fn from(val: ffi::UUID) -> Self {
        Uuid::from_u64_pair(val.high_bits, val.low_bits)
    }
}

impl Display for ffi::UUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<Uuid>::into(*self))
    }
}

impl ffi::UUID {
    pub fn now_v7() -> Self {
        let (high_bits, low_bits) = Uuid::now_v7().as_u64_pair();
        Self {
            high_bits,
            low_bits,
        }
    }

    pub fn is_nil(&self) -> bool {
        Into::<Uuid>::into(*self).is_nil()
    }
}

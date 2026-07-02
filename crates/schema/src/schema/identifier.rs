// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use thiserror::Error;

/// An identifier adhering to the grammar `[A-Za-z][A-Za-z0-9_]*`
///
/// This grammar is chosen such that a minimal amount of friction is expected
/// when interoperating across multiple programming languages and event data
/// formats.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String"))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Identifier(String);

/// Reason a string failed to parse as an [`Identifier`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdentifierError {
    #[error("identifier must not be empty")]
    Empty,
    #[error("identifier must start with an ASCII letter in [A-Za-z], found {0:?}")]
    InvalidStart(char),
    #[error("identifier character {ch:?} at byte offset {index} is not [A-Za-z0-9_]")]
    InvalidChar { ch: char, index: usize },
}

impl Identifier {
    /// Validates `s` against the grammar `[A-Za-z][A-Za-z0-9_]*` and wraps it.
    pub fn try_new(s: impl Into<String>) -> Result<Self, IdentifierError> {
        let s = s.into();
        let mut chars = s.char_indices();
        let (_, first) = chars.next().ok_or(IdentifierError::Empty)?;
        if !first.is_ascii_alphabetic() {
            return Err(IdentifierError::InvalidStart(first));
        }
        for (index, ch) in chars {
            if !(ch.is_ascii_alphanumeric() || ch == '_') {
                return Err(IdentifierError::InvalidChar { ch, index });
            }
        }
        Ok(Self(s))
    }
}

impl From<Identifier> for String {
    fn from(id: Identifier) -> Self {
        id.0
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for Identifier {
    fn eq(&self, other: &str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<&str> for Identifier {
    fn eq(&self, other: &&str) -> bool {
        self.0 == **other
    }
}

impl<T> AsRef<T> for Identifier
where
    T: ?Sized,
    <Identifier as Deref>::Target: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.deref().as_ref()
    }
}

impl Deref for Identifier {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::str::FromStr for Identifier {
    type Err = IdentifierError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl TryFrom<&str> for Identifier {
    type Error = IdentifierError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_new(s)
    }
}

impl TryFrom<String> for Identifier {
    type Error = IdentifierError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid() {
        for s in [
            "a", "Z", "foo", "Foo", "fooBar", "foo_bar", "foo123", "a_1_b_2", "x_",
        ] {
            assert!(Identifier::try_new(s).is_ok(), "expected {s:?} to be valid");
        }
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(Identifier::try_new(""), Err(IdentifierError::Empty));
    }

    #[test]
    fn compares_to_str() {
        let id = Identifier::try_new("foo").unwrap();
        assert_eq!(id, "foo");
        assert_ne!(id, "bar");
        assert!(&id == "foo");
    }

    #[test]
    fn rejects_invalid_start() {
        for (s, ch) in [("1foo", '1'), ("_foo", '_'), ("über", 'ü')] {
            assert_eq!(
                Identifier::try_new(s),
                Err(IdentifierError::InvalidStart(ch))
            );
        }
    }

    #[test]
    fn rejects_disallowed_char_after_start() {
        for (s, ch) in [("foo-bar", '-'), ("foo bar", ' '), ("café", 'é')] {
            assert_eq!(
                Identifier::try_new(s),
                Err(IdentifierError::InvalidChar { ch, index: 3 })
            );
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trips_as_bare_string() {
        let id = Identifier::try_new("foo_42").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"foo_42\"");
        assert_eq!(serde_json::from_str::<Identifier>(&json).unwrap(), id);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialize_rejects_invalid_identifier() {
        assert!(serde_json::from_str::<Identifier>("\"1bad\"").is_err());
        assert!(serde_json::from_str::<Identifier>("\"has space\"").is_err());
    }
}

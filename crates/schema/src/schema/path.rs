// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use thiserror::Error;

use crate::Identifier;

/// A nonempty absolute path of identifier segments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Path {
    namespace: Vec<Identifier>,
    name: Identifier,
}

/// Reason a path failed validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathError {
    /// No segments were provided.
    #[error("path must contain at least one segment")]
    Empty,
    /// A string contained an invalid segment.
    #[error("invalid path segment {index}: {source}")]
    InvalidSegment {
        index: usize,
        #[source]
        source: crate::schema::identifier::IdentifierError,
    },
}

impl Path {
    /// Returns the segments preceding the path name.
    pub fn namespace(&self) -> &[Identifier] {
        &self.namespace
    }

    /// Returns the final path segment.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// Returns this path with its final segment replaced.
    pub fn with_name(&self, name: Identifier) -> Self {
        Self {
            namespace: self.namespace.clone(),
            name,
        }
    }
}

impl From<Identifier> for Path {
    fn from(name: Identifier) -> Self {
        Self {
            namespace: Vec::new(),
            name,
        }
    }
}

impl TryFrom<Vec<Identifier>> for Path {
    type Error = PathError;

    fn try_from(mut segments: Vec<Identifier>) -> Result<Self, Self::Error> {
        let name = segments.pop().ok_or(PathError::Empty)?;
        Ok(Self {
            namespace: segments,
            name,
        })
    }
}

impl FromStr for Path {
    type Err = PathError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(PathError::Empty);
        }
        value
            .split("::")
            .enumerate()
            .map(|(index, segment)| {
                Identifier::try_new(segment)
                    .map_err(|source| PathError::InvalidSegment { index, source })
            })
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
    }
}

impl Display for Path {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        for segment in &self.namespace {
            write!(formatter, "{segment}::")?;
        }
        formatter.write_str(&self.name)
    }
}

impl PartialEq<str> for Path {
    fn eq(&self, other: &str) -> bool {
        let mut other_segments = other.split("::");
        self.namespace
            .iter()
            .chain(std::iter::once(&self.name))
            .all(|segment| other_segments.next().is_some_and(|other| segment == other))
            && other_segments.next().is_none()
    }
}

impl PartialEq<&str> for Path {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.namespace
            .iter()
            .chain(std::iter::once(&self.name))
            .cmp(other.namespace.iter().chain(std::iter::once(&other.name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::ident;

    #[test]
    fn constructs_and_inspects_paths() {
        let path = Path::try_from(vec![ident("Foo"), ident("Query")]).unwrap();

        assert_eq!(path.namespace(), &[ident("Foo")]);
        assert_eq!(path.name(), "Query");
        assert_eq!(path.with_name(ident("Result")).to_string(), "Foo::Result");
    }

    #[test]
    fn parses_and_formats_paths() {
        let path: Path = "Foo::Query".parse().unwrap();
        assert_eq!(path.to_string(), "Foo::Query");
        assert_eq!(Path::from(ident("Query")).to_string(), "Query");
    }

    #[test]
    fn rejects_empty_and_invalid_paths() {
        assert_eq!(Path::try_from(Vec::new()), Err(PathError::Empty));
        assert_eq!("".parse::<Path>(), Err(PathError::Empty));
        assert!(matches!(
            "Foo::".parse::<Path>(),
            Err(PathError::InvalidSegment { index: 1, .. })
        ));
        assert!(matches!(
            "Foo::bad-name".parse::<Path>(),
            Err(PathError::InvalidSegment { index: 1, .. })
        ));
    }
}

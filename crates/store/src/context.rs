// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Context selection for stores.

use smallvec::SmallVec;
use uuid::Uuid;

use crate::Error;

/// An ordered, non-empty set of context IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextSet(SmallVec<[Uuid; 1]>);

impl ContextSet {
    /// Creates a context set, removing duplicate IDs while preserving their first occurrence.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyContextSet`] when no IDs are provided.
    pub fn try_new(ids: impl IntoIterator<Item = Uuid>) -> Result<Self, Error> {
        let mut unique = SmallVec::new();
        for id in ids {
            if !unique.contains(&id) {
                unique.push(id);
            }
        }
        if unique.is_empty() {
            return Err(Error::EmptyContextSet);
        }
        Ok(Self(unique))
    }

    /// Creates a context set containing one ID.
    pub fn one(id: Uuid) -> Self {
        Self(SmallVec::from_buf([id]))
    }

    /// Returns context IDs in selection order.
    pub fn as_slice(&self) -> &[Uuid] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_set_requires_an_id_and_preserves_first_occurrence_order() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);

        assert!(matches!(
            ContextSet::try_new([]),
            Err(Error::EmptyContextSet)
        ));
        assert_eq!(
            ContextSet::try_new([first, second, first])
                .unwrap()
                .as_slice(),
            [first, second]
        );
    }
}

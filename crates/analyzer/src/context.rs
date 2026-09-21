// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Indexing of runtime contexts by the entities that contribute telemetry.

use std::collections::BTreeSet;
use std::path::Path;

use rustc_hash::FxHashMap;
use uuid::Uuid;

/// Identifies a runtime context rather than an instrumented entity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContextId(Uuid);

impl ContextId {
    /// Returns the underlying UUID for storage or transport APIs.
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for ContextId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Analysis targets discovered by cheaply scanning one runtime context.
///
/// For example, in a distributed application with one driver and multiple
/// workers, the driver and worker contexts identify the driver as their shared
/// analysis target. Only events carrying that relationship need to be read.
#[derive(Debug, Default)]
pub struct ContextInventory {
    /// The entities whose analyses include telemetry from this context.
    pub analysis_target_ids: BTreeSet<Uuid>,
}

/// Maps each analysis target to all runtime contexts contributing telemetry.
///
/// Multiple independent targets are supported, such as two implementations
/// running the same benchmark workload whose results will be compared.
#[derive(Debug, Default)]
pub struct ContextIndex {
    contexts_by_analysis_target: FxHashMap<Uuid, BTreeSet<ContextId>>,
}

impl ContextIndex {
    /// Adds the associations discovered in one runtime context.
    pub fn add_inventory(&mut self, context_id: ContextId, inventory: ContextInventory) {
        for analysis_target_id in inventory.analysis_target_ids {
            self.contexts_by_analysis_target
                .entry(analysis_target_id)
                .or_default()
                .insert(context_id);
        }
    }

    /// Returns the entities for which aggregate analysis can be requested.
    pub fn analysis_target_ids(&self) -> impl Iterator<Item = Uuid> + '_ {
        self.contexts_by_analysis_target.keys().copied()
    }

    /// Returns all contexts whose telemetry contributes to `analysis_target_id`.
    pub fn contexts_of_analysis_target(&self, analysis_target_id: Uuid) -> Vec<ContextId> {
        self.contexts_by_analysis_target
            .get(&analysis_target_id)
            .map(|contexts| contexts.iter().copied().collect())
            .unwrap_or_default()
    }
}

/// Builds an index from inventories of UUID-named context directories.
///
/// Non-directory and non-UUID entries are ignored. Each direct child of `root` is visited once.
pub fn index_contexts<E>(
    root: &Path,
    inventory: impl Fn(&Path) -> Result<ContextInventory, E>,
) -> Result<ContextIndex, E>
where
    E: From<std::io::Error>,
{
    let mut index = ContextIndex::default();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let context_dir = entry.path();
        let Some(context_id) = context_dir
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| Uuid::parse_str(name).ok())
            .map(ContextId::from)
        else {
            continue;
        };

        index.add_inventory(context_id, inventory(&context_dir)?);
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_contexts_by_analysis_target() {
        let analysis_target_id = Uuid::from_u128(1);
        let target_context = Uuid::from_u128(2);
        let child_context = Uuid::from_u128(3);
        let mut index = ContextIndex::default();

        index.add_inventory(
            target_context.into(),
            ContextInventory {
                analysis_target_ids: BTreeSet::from([analysis_target_id]),
            },
        );
        index.add_inventory(
            child_context.into(),
            ContextInventory {
                analysis_target_ids: BTreeSet::from([analysis_target_id]),
            },
        );

        assert_eq!(
            index.contexts_of_analysis_target(analysis_target_id),
            vec![target_context.into(), child_context.into()]
        );
    }

    #[test]
    fn indexes_uuid_named_context_directories() {
        let temp = tempfile::tempdir().unwrap();
        let analysis_target_id = Uuid::from_u128(1);
        let context_id = Uuid::from_u128(2);
        let context_dir = temp.path().join(context_id.to_string());
        std::fs::create_dir(&context_dir).unwrap();
        std::fs::create_dir(temp.path().join("not-a-context")).unwrap();
        std::fs::write(temp.path().join(Uuid::from_u128(3).to_string()), []).unwrap();

        let index = index_contexts(temp.path(), |actual_context_dir| -> std::io::Result<_> {
            assert_eq!(actual_context_dir, context_dir);
            Ok(ContextInventory {
                analysis_target_ids: BTreeSet::from([analysis_target_id]),
            })
        })
        .unwrap();

        assert_eq!(
            index.contexts_of_analysis_target(analysis_target_id),
            vec![context_id.into()]
        );
    }
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Indexed reference-tree relationships and borrowed traversal.
//!
//! For schemas using [`RefTreeConstraint`](crate::RefTreeConstraint), generated
//! model stores provide `build_ref_tree_index()`. Build it after loading, then
//! select a subtree with a typed entity ID. The index owns its relationships
//! and can outlive the store.
//!
//! For a generated `DemoStore` with a `Server` entity:
//!
//! ```ignore
//! let store = DemoStore::try_from_loader(&loader)?;
//! let index = store.build_ref_tree_index()?;
//! for root in store.entities::<Server>()? {
//!     let tree = index.ref_tree(root)?;
//!     for child in tree.children::<Connection>() {
//!         println!("{} {}", child.type_name(), child.entity_id().uuid());
//!     }
//! }
//! ```

use std::{any::TypeId, marker::PhantomData};

use quent_events::{EntityMarker, EventPayload, Id, Uuid};
use rustc_hash::FxHashMap as HashMap;
use thiserror::Error;

type Result<T> = std::result::Result<T, Error>;

/// An error from indexing or looking up reference-tree relationships.
#[derive(Debug, Error)]
pub enum Error {
    #[error("entity {0} is absent from the reference tree")]
    MissingEntity(Uuid),
    #[error("entity {0} occurs more than once in the reference tree")]
    DuplicateEntity(Uuid),
    #[error("entity {entity} references parent {parent} with a different entity type")]
    InvalidParentType { entity: Uuid, parent: Uuid },
}

/// Extracts parents whose declared entity types form an acyclic graph.
pub trait RefTreeEntityMarker: EntityMarker + 'static {
    /// Parent entity marker; roots use `Self` and always return `None` from
    /// [`Self::parent_id`].
    type Parent: EntityMarker + 'static;

    /// Returns the first tree-forming reference in schema field order, or
    /// `None` if absent.
    fn parent_id(payload: &Self::Payload) -> Option<Id<Self::Parent>>;
}

/// Identifies a schema-declared child type of `P`.
pub trait RefTreeChildOf<P: RefTreeEntityMarker>: RefTreeEntityMarker<Parent = P> {}

/// Validated relationships stored as dense child indices in flat arrays.
pub struct RefTreeIndex {
    positions: HashMap<Uuid, usize>,
    ids: Box<[Uuid]>,
    kinds: Box<[usize]>,
    markers: Box<[TypeId]>,
    offsets: Box<[usize]>,
    children: Box<[usize]>,
    roots: Box<[usize]>,
}

impl RefTreeIndex {
    /// Borrows the subtree at a typed entity ID without allocating.
    ///
    /// The selected entity may itself have a parent.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingEntity`] if the UUID is absent or belongs to a
    /// different entity marker.
    pub fn ref_tree<R: RefTreeEntityMarker>(&self, root: Id<R>) -> Result<RefTreeNode<'_, R>> {
        let id = root.uuid();
        let position = self.position(id).ok_or(Error::MissingEntity(id))?;
        if self.markers[self.kinds[position]] != TypeId::of::<R>() {
            return Err(Error::MissingEntity(id));
        }
        Ok(RefTreeNode {
            index: self,
            position,
            marker: PhantomData,
        })
    }

    /// Returns entities without a recorded parent, in unspecified order.
    pub fn roots(&self) -> impl ExactSizeIterator<Item = Uuid> + '_ {
        self.roots.iter().map(|&position| self.ids[position])
    }

    /// Returns direct children in unspecified order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingEntity`] if `id` is absent.
    pub fn children(&self, id: Uuid) -> Result<impl ExactSizeIterator<Item = Uuid> + '_> {
        let position = self.position(id).ok_or(Error::MissingEntity(id))?;
        Ok(self
            .child_positions(position)
            .iter()
            .map(|&child| self.ids[child]))
    }

    fn position(&self, id: Uuid) -> Option<usize> {
        self.positions.get(&id).copied()
    }

    fn entity_id(&self, position: usize) -> Uuid {
        self.ids[position]
    }

    fn child_positions(&self, position: usize) -> &[usize] {
        &self.children[self.offsets[position]..self.offsets[position + 1]]
    }
}

struct EntityType {
    marker: TypeId,
    parent: TypeId,
}

struct PendingEntity {
    id: Uuid,
    kind: usize,
    parent: Option<Uuid>,
}

/// Collects Reference Tree relationships from typed entities.
#[doc(hidden)]
#[derive(Default)]
pub struct RefTreeIndexBuilder {
    positions: HashMap<Uuid, usize>,
    entities: Vec<PendingEntity>,
    types: Vec<EntityType>,
}

impl RefTreeIndexBuilder {
    /// Adds entities of one type and their resolved parents.
    ///
    /// Discard the builder after an error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateEntity`] if a UUID has already been added.
    pub fn extend<E: RefTreeEntityMarker>(
        &mut self,
        entities: impl IntoIterator<Item = (Id<E>, Option<Id<E::Parent>>)>,
    ) -> Result<()> {
        let kind = self.types.len();
        self.types.push(EntityType {
            marker: TypeId::of::<E>(),
            parent: TypeId::of::<E::Parent>(),
        });
        for (id, parent) in entities {
            let id = id.uuid();
            match self.positions.entry(id) {
                std::collections::hash_map::Entry::Occupied(_) => {
                    return Err(Error::DuplicateEntity(id));
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(self.entities.len());
                }
            }
            self.entities.push(PendingEntity {
                id,
                kind,
                parent: parent.map(|parent| parent.uuid()),
            });
        }
        Ok(())
    }

    /// Resolves parent UUIDs and packs child adjacency in `O(n)` time and
    /// storage.
    ///
    /// # Errors
    ///
    /// Returns an error if a recorded parent is absent or has a different type
    /// from the declared parent marker.
    pub fn build(self) -> Result<RefTreeIndex> {
        let mut offsets = vec![0; self.entities.len() + 1];
        let mut parents = Vec::with_capacity(self.entities.len());
        let mut roots = Vec::new();
        for (position, entity) in self.entities.iter().enumerate() {
            let parent = if let Some(id) = entity.parent {
                let parent = self
                    .positions
                    .get(&id)
                    .copied()
                    .ok_or(Error::MissingEntity(id))?;
                if self.types[entity.kind].parent != self.types[self.entities[parent].kind].marker {
                    return Err(Error::InvalidParentType {
                        entity: entity.id,
                        parent: id,
                    });
                }
                offsets[parent + 1] += 1;
                parent
            } else {
                roots.push(position);
                usize::MAX
            };
            parents.push(parent);
        }
        for position in 1..offsets.len() {
            offsets[position] += offsets[position - 1];
        }
        let mut children = vec![0; offsets[self.entities.len()]];
        for (child, parent) in parents.into_iter().enumerate() {
            if parent != usize::MAX {
                children[offsets[parent]] = child;
                offsets[parent] += 1;
            }
        }
        // Filling advances each start offset to the end of its child range.
        // Shift those ends to restore the range boundaries without a cursor
        // array.
        for position in (1..offsets.len()).rev() {
            offsets[position] = offsets[position - 1];
        }
        offsets[0] = 0;
        Ok(RefTreeIndex {
            positions: self.positions,
            kinds: self.entities.iter().map(|entity| entity.kind).collect(),
            markers: self.types.into_iter().map(|ty| ty.marker).collect(),
            ids: self.entities.into_iter().map(|entity| entity.id).collect(),
            offsets: offsets.into_boxed_slice(),
            children: children.into_boxed_slice(),
            roots: roots.into_boxed_slice(),
        })
    }
}

/// A borrowed node with entity marker `E` in a validated Reference Tree.
pub struct RefTreeNode<'a, E: RefTreeEntityMarker> {
    index: &'a RefTreeIndex,
    position: usize,
    marker: PhantomData<fn() -> E>,
}

impl<E: RefTreeEntityMarker> Copy for RefTreeNode<'_, E> {}

impl<E: RefTreeEntityMarker> Clone for RefTreeNode<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, E: RefTreeEntityMarker> RefTreeNode<'a, E> {
    /// Returns the typed entity ID.
    pub fn entity_id(&self) -> Id<E> {
        Id::new(self.index.entity_id(self.position))
    }

    /// Returns the entity type name.
    pub fn type_name(&self) -> &'static str {
        E::Payload::NAME
    }

    /// Returns the number of direct children across all child types.
    pub fn child_count(&self) -> usize {
        self.index.child_positions(self.position).len()
    }

    /// Borrows direct children of type `C` without allocating.
    ///
    /// Examines all direct children; instance order is unspecified.
    pub fn children<C: RefTreeChildOf<E>>(&self) -> impl Iterator<Item = RefTreeNode<'a, C>> + 'a {
        let index = self.index;
        index
            .child_positions(self.position)
            .iter()
            .copied()
            .filter(|&position| index.markers[index.kinds[position]] == TypeId::of::<C>())
            .map(move |position| RefTreeNode {
                index,
                position,
                marker: PhantomData,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Marker<const KIND: u8>;
    struct Payload;

    impl quent_events::EventPayload for Payload {
        const NAME: &'static str = "Test";
    }

    impl<const KIND: u8> EntityMarker for Marker<KIND> {
        type Payload = Payload;
    }

    impl RefTreeEntityMarker for Marker<0> {
        type Parent = Self;

        fn parent_id(_: &Payload) -> Option<Id<Self::Parent>> {
            None
        }
    }

    impl RefTreeEntityMarker for Marker<1> {
        type Parent = Marker<0>;

        fn parent_id(_: &Payload) -> Option<Id<Self::Parent>> {
            None
        }
    }

    #[test]
    fn resolves_parents_added_after_children_and_keeps_trees_separate() {
        let mut builder = RefTreeIndexBuilder::default();
        builder
            .extend::<Marker<1>>([(3, 1), (4, 2), (5, 1)].map(|(child, parent)| {
                (
                    Id::new(Uuid::from_u128(child)),
                    Some(Id::new(Uuid::from_u128(parent))),
                )
            }))
            .unwrap();
        builder
            .extend::<Marker<0>>([1, 2].map(|root| (Id::new(Uuid::from_u128(root)), None)))
            .unwrap();
        let index = builder.build().unwrap();
        let mut roots = index.roots().collect::<Vec<_>>();
        roots.sort();
        assert_eq!(roots, [Uuid::from_u128(1), Uuid::from_u128(2)]);

        let children = |id| {
            let mut children = index
                .children(Uuid::from_u128(id))
                .unwrap()
                .collect::<Vec<_>>();
            children.sort();
            children
        };
        assert_eq!(children(1), [Uuid::from_u128(3), Uuid::from_u128(5)]);
        assert_eq!(children(2), [Uuid::from_u128(4)]);
        for leaf in [3, 4, 5] {
            assert!(children(leaf).is_empty());
        }
    }
}

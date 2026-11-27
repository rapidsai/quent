// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_ref_tree::analysis::Error as RefTreeError;
use quent_store::{
    Error, Id, RefTreeEntityMarker, RefTreeNode,
    event::{EventIterator, EventLoader},
};

#[allow(unused_imports, dead_code)]
mod model {
    include!(concat!(env!("OUT_DIR"), "/tree_model.rs"));
}

use model::{
    Branch, BranchEvent, EntityRef, Event, Leaf, LeafEvent, OtherRoot, OtherRootEvent, Root,
    RootEvent, TreeStore, TreeTree, TreeTreeVisitor, Uuid, ZBranch, ZBranchEvent,
};

#[derive(Default)]
struct Loader {
    roots: Vec<Event<RootEvent>>,
    other_roots: Vec<Event<OtherRootEvent>>,
    branches: Vec<Event<BranchEvent>>,
    z_branches: Vec<Event<ZBranchEvent>>,
    leaves: Vec<Event<LeafEvent>>,
}

macro_rules! loader {
    ($marker:ty, $field:ident) => {
        impl EventLoader<$marker> for Loader {
            type Error = Error;
            fn events(
                &self,
            ) -> Result<
                EventIterator<<$marker as quent_store::events::EntityMarker>::Payload, Error>,
                Error,
            > {
                Ok(Box::new(self.$field.clone().into_iter().map(Ok)))
            }
        }
    };
}

loader!(Root, roots);
loader!(OtherRoot, other_roots);
loader!(Branch, branches);
loader!(ZBranch, z_branches);
loader!(Leaf, leaves);

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn loader() -> Loader {
    Loader {
        roots: [1, 10]
            .map(|value| Event::new(id(value), 0, RootEvent::Created))
            .into(),
        other_roots: vec![Event::new(id(99), 0, OtherRootEvent::Created)],
        z_branches: [(4, 1), (40, 10)]
            .map(|(child, parent)| {
                Event::new(
                    id(child),
                    0,
                    ZBranchEvent::Created {
                        parent: EntityRef::new(id(parent), ()),
                    },
                )
            })
            .into(),
        branches: [(2, 1), (20, 10)]
            .map(|(child, parent)| {
                Event::new(
                    id(child),
                    0,
                    BranchEvent::Created {
                        parent: EntityRef::new(id(parent), ()),
                    },
                )
            })
            .into(),
        leaves: [(3, 2), (30, 20)]
            .map(|(child, parent)| {
                Event::new(
                    id(child),
                    0,
                    LeafEvent::Created {
                        parent: EntityRef::new(id(parent), ()),
                    },
                )
            })
            .into(),
    }
}

#[test]
fn typed_root_selects_one_tree_across_entity_types() {
    let store = TreeStore::try_from_loader(&loader()).unwrap();
    let index = store.build_ref_tree_index().unwrap();
    let child = store.entity(Id::<Branch>::new(id(2))).unwrap().unwrap();
    let parent: Id<Root> = child.parent_id().unwrap();
    assert_eq!(parent.uuid(), id(1));
    assert!(store.entity(parent).unwrap().is_some());
    drop(store);
    let tree = index.ref_tree(Id::<Root>::new(id(1))).unwrap();
    assert_eq!(tree.entity_id().uuid(), id(1));
    assert_eq!(tree.child_count(), 2);
    let z_branch = tree.children::<ZBranch>().next().unwrap();
    assert_eq!(z_branch.entity_id().uuid(), id(4));
    let mut branches = tree.children::<Branch>();
    let branch = branches.next().unwrap();
    assert!(branches.next().is_none());
    assert_eq!(branch.entity_id().uuid(), id(2));
    let leaf = branch.children::<Leaf>().next().unwrap();
    assert_eq!(leaf.entity_id().uuid(), id(3));
    assert_eq!(leaf.child_count(), 0);

    let other = index.ref_tree(Id::<Root>::new(id(10))).unwrap();
    let other_branch = other.children::<Branch>().next().unwrap();
    assert_eq!(other_branch.entity_id().uuid(), id(20));
    let unrelated = index.ref_tree(Id::<OtherRoot>::new(id(99))).unwrap();
    assert_eq!(unrelated.child_count(), 0);
    let subtree = index.ref_tree(Id::<Branch>::new(id(2))).unwrap();
    let subtree_leaf = subtree.children::<Leaf>().next().unwrap();
    assert_eq!(subtree_leaf.entity_id().uuid(), id(3));
    let mut roots = index.roots().collect::<Vec<_>>();
    roots.sort();
    assert_eq!(roots, [id(1), id(10), id(99)]);
}

#[test]
fn walks_child_types_in_schema_order_with_typed_callbacks() {
    struct Visitor<'a> {
        store: &'a TreeStore,
        entered: Vec<(&'static str, Uuid)>,
        left: Vec<Uuid>,
        parents: Vec<Id<Root>>,
    }

    impl TreeTreeVisitor for Visitor<'_> {
        fn visit<'a, E: RefTreeEntityMarker>(&mut self, node: RefTreeNode<'a, E>)
        where
            RefTreeNode<'a, E>: TreeTree,
        {
            self.entered
                .push((node.type_name(), node.entity_id().uuid()));
            node.walk_children(self);
            self.left.push(node.entity_id().uuid());
        }

        fn visit_branch(&mut self, node: RefTreeNode<'_, Branch>) {
            let entity = self.store.entity(node.entity_id()).unwrap().unwrap();
            self.parents.push(entity.parent_id().unwrap());
            self.visit(node);
        }
    }

    let store = TreeStore::try_from_loader(&loader()).unwrap();
    let index = store.build_ref_tree_index().unwrap();
    let mut visitor = Visitor {
        store: &store,
        entered: Vec::new(),
        left: Vec::new(),
        parents: Vec::new(),
    };
    index
        .ref_tree(Id::<Root>::new(id(1)))
        .unwrap()
        .walk(&mut visitor);
    assert_eq!(
        visitor.entered,
        [
            ("Root", id(1)),
            ("ZBranch", id(4)),
            ("Branch", id(2)),
            ("Leaf", id(3))
        ]
    );
    assert_eq!(visitor.left, [id(4), id(3), id(2), id(1)]);
    assert_eq!(visitor.parents[0].uuid(), id(1));

    visitor.entered.clear();
    visitor.left.clear();
    index
        .ref_tree(Id::<Branch>::new(id(2)))
        .unwrap()
        .walk(&mut visitor);
    assert_eq!(visitor.entered, [("Branch", id(2)), ("Leaf", id(3))]);
    assert_eq!(visitor.left, [id(3), id(2)]);
}

#[test]
fn visitor_can_skip_a_subtree() {
    #[derive(Default)]
    struct Visitor(Vec<Uuid>);

    impl TreeTreeVisitor for Visitor {
        fn visit<'a, E: RefTreeEntityMarker>(&mut self, node: RefTreeNode<'a, E>)
        where
            RefTreeNode<'a, E>: TreeTree,
        {
            self.0.push(node.entity_id().uuid());
            if node.type_name() != "Branch" {
                node.walk_children(self);
            }
        }
    }

    let store = TreeStore::try_from_loader(&loader()).unwrap();
    let index = store.build_ref_tree_index().unwrap();
    let mut visitor = Visitor::default();
    index
        .ref_tree(Id::<Root>::new(id(1)))
        .unwrap()
        .walk(&mut visitor);
    assert_eq!(visitor.0, [id(1), id(4), id(2)]);
}

#[test]
fn typed_lookup_rejects_absent_ids_and_ids_of_other_types() {
    let store = TreeStore::try_from_loader(&loader()).unwrap();
    let index = store.build_ref_tree_index().unwrap();
    for missing in [2, 99, 999] {
        assert!(
            matches!(index.ref_tree(Id::<Root>::new(id(missing))), Err(RefTreeError::MissingEntity(uuid)) if uuid == id(missing))
        );
    }
}

#[test]
fn index_building_rejects_uuid_collisions_across_types() {
    let mut loader = loader();
    loader
        .other_roots
        .push(Event::new(id(1), 0, OtherRootEvent::Created));
    let store = TreeStore::try_from_loader(&loader).unwrap();
    assert!(
        matches!(store.build_ref_tree_index(), Err(Error::RefTree(RefTreeError::DuplicateEntity(uuid))) if uuid == id(1))
    );
}

#[test]
fn index_building_rejects_missing_parents() {
    let mut loader = loader();
    loader.branches.push(Event::new(
        id(50),
        0,
        BranchEvent::Created {
            parent: EntityRef::new(id(999), ()),
        },
    ));
    let store = TreeStore::try_from_loader(&loader).unwrap();
    assert!(
        matches!(store.build_ref_tree_index(), Err(Error::RefTree(RefTreeError::MissingEntity(uuid))) if uuid == id(999))
    );
}

#[test]
fn index_building_rejects_parent_ids_of_other_entity_types() {
    for wrong_parent in [2, 3, 99] {
        let mut loader = loader();
        loader.branches[0].data = BranchEvent::Created {
            parent: EntityRef::new(id(wrong_parent), ()),
        };
        let store = TreeStore::try_from_loader(&loader).unwrap();
        assert!(matches!(
            store.build_ref_tree_index(),
            Err(Error::RefTree(RefTreeError::InvalidParentType { .. }))
        ));
    }
}

#[allow(unused_imports, dead_code)]
mod plain {
    include!(concat!(env!("OUT_DIR"), "/plain_model.rs"));
}

impl EventLoader<plain::Task> for Loader {
    type Error = Error;
    fn events(&self) -> Result<EventIterator<plain::TaskEvent, Error>, Error> {
        Ok(Box::new(std::iter::once(Ok(Event::new(
            id(1),
            7,
            plain::TaskEvent::ParentId,
        )))))
    }
}

#[test]
fn model_without_ref_tree_support_loads_and_exposes_typed_events() {
    use plain::task::TaskEvents;
    let store = plain::PlainStore::try_from_loader(&Loader::default()).unwrap();
    let entity = store
        .entity(Id::<plain::Task>::new(id(1)))
        .unwrap()
        .unwrap();
    assert_eq!(entity.parent_id().timestamp(), 7);
}

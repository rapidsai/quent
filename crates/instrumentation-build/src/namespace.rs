// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{Entity, Identifier, Record, Schema};

/// A tree of Rust namespaces containing schema records and entities.
pub(crate) struct Namespace<'schema> {
    path: Vec<Identifier>,
    records: Vec<&'schema Record>,
    entities: Vec<&'schema Entity>,
    children: Vec<Self>,
}

impl<'schema> Namespace<'schema> {
    pub(crate) fn root(schema: &'schema Schema) -> Self {
        let mut root = Self::new(Vec::new());
        for record in schema.records() {
            root.namespace_mut(record.path().namespace())
                .records
                .push(record);
        }
        for entity in schema.entities() {
            root.namespace_mut(entity.path().namespace())
                .entities
                .push(entity);
        }
        root
    }

    pub(crate) fn path(&self) -> &[Identifier] {
        &self.path
    }

    pub(crate) fn records(&self) -> &[&'schema Record] {
        &self.records
    }

    pub(crate) fn entities(&self) -> &[&'schema Entity] {
        &self.entities
    }

    pub(crate) fn children(&self) -> &[Self] {
        &self.children
    }

    /// Returns child namespaces containing entities directly or transitively.
    pub(crate) fn children_with_entities(&self) -> impl Iterator<Item = &Self> {
        self.children.iter().filter(|child| child.has_entities())
    }

    pub(crate) fn has_entities(&self) -> bool {
        !self.entities.is_empty() || self.children.iter().any(Self::has_entities)
    }

    pub(crate) fn all_entities(&self) -> Vec<&'schema Entity> {
        let mut entities = self.entities.clone();
        for child in &self.children {
            entities.extend(child.all_entities());
        }
        entities
    }

    pub(crate) fn descendants_with_entities(&self) -> Vec<&Self> {
        let mut descendants = Vec::new();
        for child in self.children_with_entities() {
            descendants.push(child);
            descendants.extend(child.descendants_with_entities());
        }
        descendants
    }

    fn new(path: Vec<Identifier>) -> Self {
        Self {
            path,
            records: Vec::new(),
            entities: Vec::new(),
            children: Vec::new(),
        }
    }

    fn namespace_mut(&mut self, path: &[Identifier]) -> &mut Self {
        let mut namespace = self;
        for segment in path {
            let index = match namespace
                .children
                .iter()
                .position(|child| child.path.last() == Some(segment))
            {
                Some(index) => index,
                None => {
                    let mut child_path = namespace.path.clone();
                    child_path.push(segment.clone());
                    namespace.children.push(Self::new(child_path));
                    namespace.children.len() - 1
                }
            };
            namespace = &mut namespace.children[index];
        }
        namespace
    }
}

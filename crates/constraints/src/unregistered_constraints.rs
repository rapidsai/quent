// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

use quent_schema::visitor::{Cursor, Element, Visitor};
use rustc_hash::FxHashSet;

/// Utility visitor to collect any unregistered constraint names.
pub(crate) struct UnregisteredConstraints {
    registered: FxHashSet<&'static str>,
    unregistered: BTreeSet<String>,
}

impl UnregisteredConstraints {
    pub(crate) fn new(registered: &'static [&'static str]) -> Self {
        Self {
            registered: registered.iter().copied().collect(),
            unregistered: BTreeSet::new(),
        }
    }
}

impl Visitor for UnregisteredConstraints {
    type Output = BTreeSet<String>;
    fn visit(&mut self, cursor: &Cursor) {
        if let Element::Annotations(annotations) = cursor.current() {
            for constraint in annotations.constraints() {
                if !self.registered.contains(constraint.name()) {
                    self.unregistered.insert(constraint.name().to_string());
                }
            }
        }
    }
    fn finish(self) -> Self::Output {
        self.unregistered
    }
}

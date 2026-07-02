// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::schema::{Map, annotations::Annotations, field::Field, identifier::Identifier};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Record {
    /// The name of the record.
    name: Identifier,
    /// The fields of the record.
    #[cfg_attr(feature = "ts", ts(as = "indexmap::IndexMap<Identifier, Field>"))]
    fields: Map<Identifier, Field>,
    /// Annotations of this record.
    annotations: Annotations,
}

impl Record {
    pub(crate) fn from_parts(
        name: Identifier,
        fields: Map<Identifier, Field>,
        annotations: Annotations,
    ) -> Self {
        Self {
            name,
            fields,
            annotations,
        }
    }

    /// The name of the record.
    pub fn name(&self) -> &Identifier {
        &self.name
    }

    /// The annotations of this record.
    pub fn annotations(&self) -> &Annotations {
        &self.annotations
    }

    /// The field declared under `name`, if any.
    pub fn field(&self, name: &Identifier) -> Option<&Field> {
        self.fields.get(name)
    }

    /// The declared fields, in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = &Field> + '_ {
        self.fields.values()
    }
}

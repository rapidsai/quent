// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::schema::{annotations::Annotations, identifier::Identifier};

/// Types of data values in [`crate::event::Event`]s and [`crate::record::Record`]s.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum DataType {
    Bool,
    Uuid,
    String,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Option(Box<DataType>),
    List(Box<DataType>),
    /// A reference to a named [`crate::record::Record`].
    ///
    /// The top-level [`crate::Schema::records`] field must contain a
    /// [`crate::record::Record`] with this name, otherwise the
    /// [`crate::Schema`] is ill-formed.
    Record(Identifier),
    /// A record whose fields are determined by the instrumentation client at
    /// run-time.
    DynamicRecord,
    /// A reference to an entity, optionally carrying data and annotations.
    EntityRef {
        /// Optional payload data carried by the reference.
        data: Option<Box<DataType>>,
        /// Annotations of this entity reference.
        annotations: Annotations,
    },
}

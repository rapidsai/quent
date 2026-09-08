// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::Path;
use crate::schema::annotations::Annotations;

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
    /// A reference to the [`crate::Record`] declared at the exact path.
    ///
    /// The [`crate::Schema`] is ill-formed if no record is declared at this path.
    Record(Path),
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

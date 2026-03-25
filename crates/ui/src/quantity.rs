// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Quantity specifications for resource capacities.
//!
//! A [`QuantitySpec`] describes how to display values of a particular quantity
//! type (e.g. bytes, seconds) including the base unit symbol, singular/plural
//! forms, and which prefix system to use for occupancy vs. rate capacities.

use serde::Serialize;
use ts_rs::TS;

/// The prefix system used for scaling and abbreviating a quantity.
#[derive(TS, Serialize, Clone, Copy, Debug)]
pub enum PrefixSystem {
    /// SI decimal prefixes (k, M, G, T — powers of 1000).
    Si,
    /// IEC binary prefixes (Ki, Mi, Gi, Ti — powers of 1024).
    Iec,
    /// No prefix scaling.
    None,
}

/// The kind of capacity of a resource.
#[derive(TS, Serialize, Clone, Copy, Debug, Default)]
pub enum CapacityKind {
    /// The value represents the amount of resource capacity held/occupied
    /// during a span.
    #[default]
    Occupancy,
    /// The value represents the total quantity of work performed over a span.
    Rate,
}

/// A capacity declaration for a resource type, as exposed to the UI.
#[derive(TS, Serialize, Clone, Debug)]
pub struct CapacityDecl {
    /// The name of this capacity.
    pub name: String,
    /// The kind of capacity.
    pub kind: CapacityKind,
    /// The name of the quantity spec (key into the quantity_specs map).
    pub quantity: String,
}

impl Default for CapacityDecl {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: CapacityKind::default(),
            quantity: "unit".into(),
        }
    }
}

/// Specification of how to display a quantity with units.
#[derive(TS, Serialize, Clone, Debug)]
pub struct QuantitySpec {
    /// Base unit symbol, e.g. "B" for bytes.
    pub symbol: String,
    /// Singular form, e.g. "byte".
    pub singular: String,
    /// Plural form, e.g. "bytes".
    pub plural: String,
    /// Prefix system for occupancy display.
    pub occupancy_prefix: PrefixSystem,
    /// Prefix system for rate display.
    pub rate_prefix: PrefixSystem,
}

impl QuantitySpec {
    /// Bytes: symbol "B", IEC for occupancy, SI for rate.
    pub fn bytes() -> Self {
        Self {
            symbol: "B".into(),
            singular: "byte".into(),
            plural: "bytes".into(),
            occupancy_prefix: PrefixSystem::Iec,
            rate_prefix: PrefixSystem::Si,
        }
    }

    /// Seconds: symbol "s", SI for both occupancy and rate.
    pub fn seconds() -> Self {
        Self {
            symbol: "s".into(),
            singular: "second".into(),
            plural: "seconds".into(),
            occupancy_prefix: PrefixSystem::Si,
            rate_prefix: PrefixSystem::Si,
        }
    }

    /// Unitless: no symbol, no prefix scaling.
    pub fn unit() -> Self {
        Self {
            symbol: String::new(),
            singular: "unit".into(),
            plural: "units".into(),
            occupancy_prefix: PrefixSystem::None,
            rate_prefix: PrefixSystem::None,
        }
    }
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Detects the canonical NVTX sink for capture bindings.

use quent_constraints::Constraint;
use quent_nvtx::{NvtxConstraint, nvtx_path};
use quent_schema::{Entity, Schema};

pub(crate) fn is_sink(entity: &Entity) -> bool {
    entity.path() == &nvtx_path() && entity.annotations().has_constraint(NvtxConstraint::NAME)
}

pub(crate) fn enabled(schema: &Schema) -> bool {
    schema.entities().any(is_sink)
}

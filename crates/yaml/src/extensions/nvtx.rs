// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The model-level NVTX schema extension selected by `nvtx: true`.

use quent_schema::{Path, Schema};

use crate::ast::Model;
use crate::diag::Diagnostics;

/// Selection gathered before core lowering and applied after the schema exists.
pub(crate) struct Elaborator {
    process: Option<(String, Path)>,
}

impl Elaborator {
    pub(crate) fn new(model: &Model, sink: &mut Diagnostics) -> Self {
        let mut process = None;
        for (name, entity) in &model.entities {
            if !entity.nvtx {
                continue;
            }

            let location = format!("entities.{name}.nvtx");
            if process.is_some() {
                sink.error(
                    &location,
                    "NVTX may be enabled on only one process entity type",
                    Some(
                        "keep `nvtx: true` on the entity that owns this model's NVTX stream".into(),
                    ),
                );
                continue;
            }

            match name.parse::<Path>() {
                Ok(path) => process = Some((location, path)),
                Err(error) => sink.error(
                    &location,
                    format!("invalid NVTX process entity name `{name}`: {error}"),
                    None,
                ),
            }
        }
        Self { process }
    }

    pub(crate) fn elaborate(&self, schema: Schema, sink: &mut Diagnostics) -> Option<Schema> {
        let Some((location, process)) = &self.process else {
            return Some(schema);
        };
        match nvtx_schema::compose(&schema, process) {
            Ok(schema) => Some(schema),
            Err(error) => {
                sink.error(location, error.to_string(), None);
                None
            }
        }
    }
}

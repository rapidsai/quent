// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Canonical NVTX schema integration.

use quent_nvtx::{NvtxEntityBuilder, canonical_records};
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Record, Schema,
    builder::{AnnotationsBuilder, RecordBuilder},
};

use crate::{ast::Model, diag::Diagnostics, lower::build_or_diagnose};

pub(super) fn elaborate(model: &Model, sink: &mut Diagnostics) -> Option<(Entity, Vec<Record>)> {
    if !model.integrations.nvtx {
        return None;
    }
    let entity = build_or_diagnose(
        NvtxEntityBuilder::new()
            .with_annotations(rust_type(&Annotations::default(), "SinkEvent"))
            .build(),
        "nvtx",
        sink,
    )?;
    let records = canonical_records()
        .into_iter()
        .map(|record| {
            RecordBuilder::new(record.path().clone())
                .with_fields(record.fields().cloned())
                .with_annotations(rust_type(record.annotations(), record.path().name()))
                .build()
                .expect("canonical NVTX record with Rust metadata is valid")
        })
        .collect();
    Some((entity, records))
}

fn rust_type(annotations: &Annotations, name: &str) -> Annotations {
    AnnotationsBuilder::from_annotations(annotations)
        .with_metadata(
            "quent.codegen.rust.type",
            Some(format!("::quent_nvtx_sink::quent::nvtx::{name}")),
        )
        .build()
        .expect("canonical NVTX Rust metadata is valid")
}

pub(super) fn validate_processes(schema: &Schema, model: &Model, sink: &mut Diagnostics) {
    if !model.integrations.nvtx {
        return;
    }
    let has_process = schema.entities().any(|entity| {
        entity.events().any(|event| {
            event.cardinality() == Cardinality::Once
                && event
                    .fields()
                    .any(|field| field.ty() == &DataType::Record(quent_os::process_path()))
        })
    });
    if !has_process {
        sink.error(
            "integrations.nvtx",
            "NVTX integration requires at least one entity representing an OS process",
            None,
        );
    }
}

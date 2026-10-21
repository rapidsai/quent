// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use quent_events_build::{Options, Runtime, generate_str};
use quent_nvtx::{NvtxConstraint, NvtxEntityBuilder, canonical_records};
use quent_schema::builder::SchemaBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let schema = SchemaBuilder::try_new("NvtxEvents")?
        .with_entity(NvtxEntityBuilder::new().build()?)
        .with_records(canonical_records())
        .build()?;
    schema.walk(NvtxConstraint::default())?;
    let source = generate_str(
        &schema,
        &Options {
            serde: true,
            event_derives: &["Clone"],
            record_derives: &["Clone", "PartialEq", "Eq", "Default"],
            ..Options::default()
        },
        &Runtime {
            events: syn::parse_quote!(::quent_events),
            entity_marker: syn::parse_quote!(::quent_events::EntityMarker),
            serde: None,
        },
    )?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    std::fs::write(out_dir.join("nvtx_events.rs"), source)?;
    Ok(())
}

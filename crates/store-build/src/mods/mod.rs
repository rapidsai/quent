// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Generates analysis support from schema constraints.

use proc_macro2::TokenStream;
use quent_schema::Schema;

#[cfg(feature = "ui")]
mod attribute_projection;
#[cfg(feature = "analysis")]
mod log;

/// Chooses which analysis features to generate.
///
/// These options apply even when entity stores are disabled.
#[derive(Debug, Clone, Copy)]
pub struct SemanticModuleOptions {
    /// Generate analysis support for entities with the log constraint.
    ///
    /// Enabled by default. Requires the `analysis` feature. The `ui` feature
    /// also enables attribute conversion.
    pub log_analysis: bool,
}

impl Default for SemanticModuleOptions {
    fn default() -> Self {
        Self { log_analysis: true }
    }
}

pub(super) fn generate(schema: &Schema, opts: &SemanticModuleOptions) -> TokenStream {
    #[cfg(feature = "analysis")]
    if opts.log_analysis {
        return log::generate(schema);
    }
    let _ = (schema, opts);
    TokenStream::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntityStoreImplementations, Options, generate_str};
    use quent_log::{LevelDecl, LogEntityBuilder};
    use quent_schema::{
        Annotations,
        test_utils::{entity, event, ident, path, schema},
    };

    #[test]
    fn log_analysis_defaults_on_and_is_independent_of_native_stores() {
        let log = LogEntityBuilder::new(path("Nested::Log"))
            .with_levels(["fine", "critical"].map(|name| LevelDecl {
                name: ident(name),
                annotations: Annotations::default(),
                attributes: Vec::new(),
            }))
            .build()
            .unwrap();
        let schema = schema("Demo", [log], []);
        assert!(SemanticModuleOptions::default().log_analysis);
        for native in [true, false] {
            let options = Options {
                entity_store: EntityStoreImplementations { native },
                filesystem: false,
                ..Options::default()
            };
            let source = generate_str(&schema, &options).unwrap();
            assert_eq!(
                source.contains("LogEventPayload for nested::LogEvent"),
                cfg!(feature = "analysis")
            );
            assert_eq!(
                source.contains("LogAttributes for nested::LogEvent"),
                cfg!(feature = "ui")
            );
            let source = generate_str(
                &schema,
                &Options {
                    mods: SemanticModuleOptions {
                        log_analysis: false,
                    },
                    ..options
                },
            )
            .unwrap();
            assert!(!source.contains("::quent_log::"));
        }
        let ordinary = schema_of_ordinary_entity();
        assert!(
            !generate_str(&ordinary, &Options::default())
                .unwrap()
                .contains("::quent_log::")
        );
    }

    fn schema_of_ordinary_entity() -> quent_schema::Schema {
        schema("Demo", [entity("Log", [event("info", [])])], [])
    }

    #[test]
    fn generation_rejects_invalid_log_constraints_even_when_analysis_is_disabled() {
        use quent_schema::builder::{AnnotationsBuilder, EntityBuilder};
        let log = EntityBuilder::new(path("Log"))
            .with_event(event("info", []))
            .with_annotations(
                AnnotationsBuilder::new()
                    .with_constraint(
                        quent_log::LogDefinition::NAME,
                        Some(r#"{"levels":["info"]}"#.into()),
                    )
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        let schema = schema("Demo", [log], []);
        let error = generate_str(
            &schema,
            &Options {
                mods: SemanticModuleOptions {
                    log_analysis: false,
                },
                ..Options::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("log validation failed"));
    }
}

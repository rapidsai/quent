// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Resource forms and their schema elaboration.

use indexmap::IndexMap;
use quent_resource::{Capacity, Resource, ResourceBuilder};
use quent_schema::builder::AnnotationsBuilder;
use quent_schema::{Annotations, DataType, Field, Identifier, Path, Record};
use serde::Deserialize;

use crate::ast::Model;
use crate::diag::Diagnostics;
use crate::extensions::reference::entity_ref_type;
use crate::lower::ident;

/// A resource declaration.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ResourceDecl {
    Unit(bool),
    Detailed(ResourceSpec),
    Capacities(IndexMap<String, CapacitySpec>),
}

/// A resource declaration with generated record name overrides.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResourceSpec {
    #[serde(default)]
    pub(crate) capacities: IndexMap<String, CapacitySpec>,
    #[serde(default, rename = "usage-record")]
    pub(crate) usage_record: Option<String>,
    #[serde(default, rename = "bounds-record")]
    pub(crate) bounds_record: Option<String>,
}

impl ResourceDecl {
    fn usage_record(&self) -> Option<&str> {
        match self {
            Self::Detailed(spec) => spec.usage_record.as_deref(),
            _ => None,
        }
    }

    fn bounds_record(&self) -> Option<&str> {
        match self {
            Self::Detailed(spec) => spec.bounds_record.as_deref(),
            _ => None,
        }
    }
}

/// One capacity of a resource.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacitySpec {
    pub(crate) kind: quent_resource::CapacityKind,
    #[serde(default, rename = "known-bounds")]
    pub(crate) known_bounds: bool,
}

/// Marks an event attribute as carrying the resource's bounds.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ResourceBoundsField {
    pub(crate) sets_resource_bounds: bool,
}

/// A resource usage reference.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UsesForm {
    pub(crate) uses: String,
}

#[derive(Default)]
pub(crate) struct Elaborator {
    usage_records: IndexMap<String, Path>,
}

impl Elaborator {
    pub(crate) fn new(model: &Model, sink: &mut Diagnostics) -> Self {
        let mut elaborator = Self::default();
        for (name, entity) in &model.entities {
            if let Some(resource) = &entity.resource {
                elaborator.add_usage_record(
                    name,
                    resource,
                    &format!("entities.{name}.resource"),
                    sink,
                );
            }
        }
        for (name, fsm) in &model.fsms {
            if let Some(resource) = &fsm.resource {
                elaborator.add_usage_record(name, resource, &format!("fsms.{name}.resource"), sink);
            }
        }
        elaborator
    }

    fn add_usage_record(
        &mut self,
        owner: &str,
        resource: &ResourceDecl,
        path: &str,
        sink: &mut Diagnostics,
    ) {
        let record = match resource.usage_record() {
            Some(name) => ident(name, &format!("{path}.usage-record"), sink).map(Path::from),
            None => ident(owner, path, sink)
                .map(|owner| ResourceBuilder::default_usage_record_path(&owner.into())),
        };
        if let Some(record) = record {
            self.usage_records
                .entry(owner.to_string())
                .or_insert(record);
        }
    }

    pub(crate) fn elaborate(
        &self,
        owner: &str,
        id: Identifier,
        resource: &ResourceDecl,
        path: &str,
        sink: &mut Diagnostics,
    ) -> (Vec<Record>, Option<Path>, Option<String>) {
        let resource_path = format!("{path}.resource");
        let Some(usage_record_name) = self.usage_records.get(owner).cloned() else {
            return (Vec::new(), None, None);
        };
        let bounds_record_name = match resource.bounds_record() {
            Some(name) => {
                ident(name, &format!("{resource_path}.bounds-record"), sink).map(Path::from)
            }
            None => Some(ResourceBuilder::default_bounds_record_path(
                &id.clone().into(),
            )),
        };
        let Some(bounds_record_name) = bounds_record_name else {
            return (Vec::new(), None, None);
        };

        let mut builder =
            ResourceBuilder::with_record_names(id, usage_record_name, bounds_record_name);
        let capacities = match resource {
            ResourceDecl::Unit(true) => None,
            ResourceDecl::Unit(false) => {
                sink.error(
                    &resource_path,
                    "write `resource: true` for a unit resource, or list capacities",
                    None,
                );
                return (Vec::new(), None, None);
            }
            ResourceDecl::Capacities(capacities) => Some(capacities),
            ResourceDecl::Detailed(spec) => Some(&spec.capacities),
        };
        if let Some(capacities) = capacities {
            for (name, capacity) in capacities {
                if let Some(name) = ident(name, &resource_path, sink) {
                    builder = builder
                        .with_capacity(name, Capacity::new(capacity.kind, capacity.known_bounds));
                }
            }
        }

        match builder.build() {
            Ok(parts) => {
                let constraint = match parts.definition.constraint_data() {
                    Ok(data) => Some(data),
                    Err(error) => {
                        sink.error(&resource_path, error.to_string(), None);
                        None
                    }
                };
                let bounds = parts.bounds.as_ref().map(|record| record.path().clone());
                let mut records = vec![parts.usage];
                records.extend(parts.bounds);
                (records, bounds, constraint)
            }
            Err(error) => {
                sink.error(&resource_path, error.to_string(), None);
                (Vec::new(), None, None)
            }
        }
    }

    fn usage_record(&self, target: &str, path: &str, sink: &mut Diagnostics) -> Option<Path> {
        if let Some(record) = self.usage_records.get(target) {
            return Some(record.clone());
        }
        let target = ident(target, path, sink)?;
        sink.error(
            path,
            format!("`{target}` does not declare a resource"),
            Some("`uses` must name an entity or FSM with a `resource:` declaration".to_string()),
        );
        None
    }
}

/// Build a field carrying the resource's generated bounds record.
pub(crate) fn bounds_field(
    name: Option<Identifier>,
    bounds_record: Option<&Path>,
    path: &str,
    sink: &mut Diagnostics,
) -> Option<Field> {
    match bounds_record {
        Some(record) => Some(Field::new(
            name?,
            DataType::Record(record.clone()),
            Annotations::default(),
        )),
        None => {
            sink.error(
                path,
                "`sets-resource-bounds` needs a resource with a capacity whose bounds are known",
                None,
            );
            None
        }
    }
}

pub(crate) fn attach_constraint(builder: AnnotationsBuilder, data: String) -> AnnotationsBuilder {
    builder.with_constraint(Resource::NAME, Some(data))
}

/// Build a reference claiming capacity from `target`.
pub(crate) fn usage_ref(
    target: &str,
    path: &str,
    resources: &Elaborator,
    sink: &mut Diagnostics,
) -> Option<DataType> {
    let usage = resources.usage_record(target, path, sink)?;
    entity_ref_type(target, Some(DataType::Record(usage)), false, path, sink)
}

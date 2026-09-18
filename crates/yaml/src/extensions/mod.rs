// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Statically composed extensions to the core YAML DSL.
//!
//! Here, an extension is built-in YAML syntax that does more than map one to
//! one to a core schema element. Elaborating an extension may generate schema
//! elements, attach one or more constraints, or both. The extension set is a
//! fixed part of this crate; there is no registration, discovery, dynamic
//! loading, or third-party plugin API.
//!
//! Core lowering enters the fixed extension set through [`Elaborator`] and
//! merges the returned schema values.

use quent_constraints::{Constraint, validate};
use quent_fsm::FsmConstraint;
use quent_ref_target::RefTargetConstraint;
use quent_ref_tree::RefTreeConstraint;
use quent_resource::{Resource, ResourceConstraint};
use quent_schema::builder::AnnotationsBuilder;
use quent_schema::{DataType, Entity, Field, Identifier, Path, Record, Schema};

use crate::ast::{self, AnnotationMap, Model, TypeExpr};
use crate::diag::{Diagnostic, Diagnostics};

pub(crate) mod fsm;
pub(crate) mod reference;
pub(crate) mod resource;

/// Elaborates built-in DSL forms into schema values.
pub(crate) struct Elaborator {
    resources: resource::Elaborator,
}

pub(crate) struct EntityElaboration {
    pub(crate) annotations: AnnotationsBuilder,
    pub(crate) records: Vec<Record>,
    pub(crate) event_context: EventContext,
}

pub(crate) struct ModelElaboration {
    pub(crate) entities: Vec<Entity>,
    pub(crate) records: Vec<Record>,
}

pub(crate) struct EventContext {
    resource_bounds_record: Option<Path>,
}

pub(crate) enum FieldContext<'a> {
    Event(&'a EventContext),
    Other,
}

pub(crate) enum FieldElaboration {
    NotHandled,
    Handled(Box<Field>),
    Rejected,
}

impl Elaborator {
    pub(crate) fn new(model: &Model, sink: &mut Diagnostics) -> Self {
        Self {
            resources: resource::Elaborator::new(model, sink),
        }
    }

    pub(crate) fn elaborate_model(
        &self,
        model: &Model,
        sink: &mut Diagnostics,
    ) -> ModelElaboration {
        let mut entities = Vec::new();
        let mut records = Vec::new();
        for (name, spec) in &model.fsms {
            if model.entities.contains_key(name) {
                sink.error(
                    &format!("fsms.{name}"),
                    format!("`{name}` is declared as both an entity and an FSM"),
                    None,
                );
                continue;
            }
            if let Some((entity, generated)) = fsm::elaborate(name, spec, self, sink) {
                entities.push(entity);
                records.extend(generated);
            }
        }
        ModelElaboration { entities, records }
    }

    pub(crate) fn elaborate_entity(
        &self,
        name: &str,
        id: Option<Identifier>,
        entity: &ast::Entity,
        annotations: AnnotationsBuilder,
        path: &str,
        sink: &mut Diagnostics,
    ) -> EntityElaboration {
        self.elaborate_entity_resource(name, id, entity.resource.as_ref(), annotations, path, sink)
    }

    pub(super) fn elaborate_entity_resource(
        &self,
        name: &str,
        id: Option<Identifier>,
        resource: Option<&resource::ResourceDecl>,
        mut annotations: AnnotationsBuilder,
        path: &str,
        sink: &mut Diagnostics,
    ) -> EntityElaboration {
        let (records, bounds_record, constraint) = match (id, resource) {
            (Some(id), Some(resource)) => self.resources.elaborate(name, id, resource, path, sink),
            _ => (Vec::new(), None, None),
        };
        if let Some(data) = constraint {
            annotations = resource::attach_constraint(annotations, data);
        }
        EntityElaboration {
            annotations,
            records,
            event_context: EventContext {
                resource_bounds_record: bounds_record,
            },
        }
    }

    pub(crate) fn elaborate_field(
        &self,
        id: &Option<Identifier>,
        field: &ast::Field,
        context: FieldContext<'_>,
        path: &str,
        sink: &mut Diagnostics,
    ) -> FieldElaboration {
        let ast::Field::ResourceBounds(marker) = field else {
            return FieldElaboration::NotHandled;
        };
        let FieldContext::Event(context) = context else {
            sink.error(
                path,
                "`sets-resource-bounds` is only valid in a resource event attribute",
                None,
            );
            return FieldElaboration::Rejected;
        };
        if !marker.sets_resource_bounds {
            sink.error(path, "`sets-resource-bounds` must be `true`", None);
            return FieldElaboration::Rejected;
        }
        match resource::bounds_field(
            id.clone(),
            context.resource_bounds_record.as_ref(),
            path,
            sink,
        ) {
            Some(field) => FieldElaboration::Handled(Box::new(field)),
            None => FieldElaboration::Rejected,
        }
    }

    pub(crate) fn elaborate_type(
        &self,
        expr: &TypeExpr,
        path: &str,
        sink: &mut Diagnostics,
    ) -> Option<DataType> {
        match expr {
            TypeExpr::Ref(form) => {
                reference::elaborate(&form.r#ref, form.data.as_deref(), false, path, self, sink)
            }
            TypeExpr::Scope(form) => reference::elaborate(
                &form.scope_ref,
                form.data.as_deref(),
                true,
                path,
                self,
                sink,
            ),
            TypeExpr::Uses(form) => resource::usage_ref(&form.uses, path, &self.resources, sink),
            _ => unreachable!("core type expressions are lowered by the core lowerer"),
        }
    }

    pub(crate) fn elaborate_annotations(
        &self,
        mut builder: AnnotationsBuilder,
        annotations: &AnnotationMap,
        path: &str,
        sink: &mut Diagnostics,
    ) -> AnnotationsBuilder {
        for (name, value) in annotations {
            let error = if name == FsmConstraint::NAME {
                Some("the FSM constraint is set from an `fsms:` block, not written directly")
            } else if name == Resource::NAME {
                Some(
                    "the resource constraint is set from a `resource:` block, not written directly",
                )
            } else {
                None
            };
            if let Some(error) = error {
                sink.error(path, error, None);
            } else {
                builder = builder.with_constraint(name, value.clone());
            }
        }
        builder
    }
}

pub(crate) fn validate_schema(schema: &Schema, sink: &mut Diagnostics) -> Option<Vec<Diagnostic>> {
    let report = validate::<(
        RefTargetConstraint,
        RefTreeConstraint,
        FsmConstraint,
        ResourceConstraint,
    )>(schema);
    if let Err(error) = report.base_constraints {
        for entity in error.entities_without_events {
            sink.error(
                &format!("entities.{entity}"),
                format!("entity `{entity}` declares no events"),
                Some("entities must declare at least one event".to_string()),
            );
        }
        for record in error.recursive_records {
            sink.error(
                &format!("records.{record}"),
                format!("record `{record}` is recursive"),
                Some(
                    "records cannot contain themselves, directly or through other records"
                        .to_string(),
                ),
            );
        }
        for reference in error.invalid_references {
            sink.error("", format!("unresolved reference: {reference}"), None);
        }
    }

    let (ref_target, ref_tree, fsm, resource) = report.results;
    for result in [
        ref_target.map_err(|error| error.to_string()),
        ref_tree.map_err(|error| error.to_string()),
        fsm.map_err(|error| error.to_string()),
        resource.map_err(|error| error.to_string()),
    ] {
        if let Err(error) = result {
            sink.error("", error, None);
        }
    }
    if sink.has_errors() {
        return None;
    }

    Some(
        report
            .unregistered_constraints
            .into_iter()
            .map(|name| {
                sink.make(
                    "",
                    format!("constraint `{name}` has no registered validator"),
                    Some("it is passed through without validation".to_string()),
                )
            })
            .collect(),
    )
}

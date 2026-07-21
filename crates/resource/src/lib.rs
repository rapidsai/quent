// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! The Quent built-in resource constraint.

use quent_constraints::{Constraint, utils::bullet_list};
use quent_fsm::FsmConstraint;
use quent_schema::{
    Annotations, DataType, Entity, Identifier,
    visitor::{Cursor, Element, Visitor},
};
use rustc_hash::{FxHashMap as Map, FxHashSet};
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod builder;

pub use builder::{BuildError, ResourceBuilder, ResourceParts};

/// A resource is an [`Entity`] with [`Capacity`] values that other entities may
/// claim through a usage over a span of time.
///
/// Every usage must end. This constraint can currently enforce that only for
/// finite-state machine (FSM) entities: leaving a state through a transition ends
/// its usages, and the exit state cannot hold attributes. Usages by other entities
/// are rejected.
///
/// Resource definitions, usages, and bounds are validated together. Usage records
/// claim capacities through entity references. Bounds records define the bound
/// values carried by events on the resource entity.
///
/// ## Requirements
///
/// 1. A resource is an entity with at least one [`Capacity`].
/// 2. [`Capacity`] identifiers are unique within a resource.
/// 3. If and only if any of the resource's capacities have a bound, the
///    resource entity has at least one event (the "bounds event") which
///    declares the bounds of all capacities that are bounded.
/// 4. An entity can use some quantity of a resource's capacities if and
///    only if it is an FSM.
/// 5. The resource named by a usage or bounds is a declared resource.
/// 6. A usage claims only capacities declared by its resource.
/// 7. A usage record is used only as the data carried by an entity reference.
/// 8. A bounds record is used only by events of the resource it names.
#[derive(Default)]
pub struct ResourceConstraint {
    errors: Vec<ResourceError>,
    resources: Map<Identifier, Map<Identifier, bool>>,
    usage_records: Map<Identifier, UsageRecord>,
    bounds_records: Map<Identifier, BoundsRecord>,
    record_refs: Vec<RecordRef>,
}

/// A resource quantity that can be claimed by a usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capacity {
    kind: CapacityKind,
    bounded: bool,
}

impl Capacity {
    /// Create a new capacity.
    pub fn new(kind: CapacityKind, bounded: bool) -> Self {
        Self { kind, bounded }
    }

    /// Return how values are interpreted over a usage span.
    pub fn kind(&self) -> CapacityKind {
        self.kind
    }

    /// Return whether the resource declares an upper bound for this capacity.
    pub fn is_bounded(&self) -> bool {
        self.bounded
    }
}

/// Defines how a capacity value is interpreted over a usage span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityKind {
    /// A quantity held throughout the usage span.
    Occupancy,
    /// A total quantity processed during the usage span.
    ///
    /// Dividing the value by the span duration yields the perceived rate.
    Rate,
}

// Map keys enforce capacity-name uniqueness (requirement 2).
type Capacities = indexmap::IndexMap<Identifier, Capacity>;

/// Payload of the `quent.resource.v0.1.0` constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    /// Declares the capacities provided by the annotated entity.
    Definition(Capacities),
    /// Declares the annotated record as bounds for `resource`.
    Bounds { resource: Identifier },
    /// Declares the annotated record as a usage of `resource`.
    Usage { resource: Identifier },
}

impl Resource {
    /// Constraint identifier.
    pub const NAME: &'static str = "quent.resource.v0.1.0";

    /// Return the declared capacity names and definitions.
    ///
    /// Return `None` unless this is [`Self::Definition`].
    pub fn capacities(&self) -> Option<impl ExactSizeIterator<Item = (&Identifier, &Capacity)>> {
        match self {
            Resource::Definition(capacities) => Some(capacities.iter()),
            _ => None,
        }
    }

    fn variant_name(&self) -> &'static str {
        match self {
            Resource::Definition(_) => "definition",
            Resource::Usage { .. } => "usage",
            Resource::Bounds { .. } => "bounds",
        }
    }
}

struct UsageRecord {
    resource: Identifier,
    claims: Vec<Identifier>,
    location: String,
}

struct BoundsRecord {
    resource: Identifier,
    fields: Vec<Identifier>,
    location: String,
}

struct RecordRef {
    record: Identifier,
    on_entity_ref: bool,
    entity: Option<(Identifier, bool)>,
    location: String,
}

impl Visitor for ResourceConstraint {
    type Output = Result<(), ResourceError>;

    fn visit(&mut self, cursor: &Cursor) {
        match cursor.current() {
            Element::Entity(entity) => {
                match self.decode_resource_annotation(cursor, entity.annotations()) {
                    Some(Resource::Definition(capacities)) => {
                        self.validate_definition(entity.name(), &capacities);
                        self.resources.insert(
                            entity.name().clone(),
                            capacities
                                .iter()
                                .map(|(name, capacity)| (name.clone(), capacity.is_bounded()))
                                .collect(),
                        );
                    }
                    Some(role @ (Resource::Usage { .. } | Resource::Bounds { .. })) => {
                        self.errors.push(ResourceError::MisplacedRole {
                            location: cursor.to_string(),
                            role: role.variant_name(),
                            element: "an entity",
                        });
                    }
                    None => {}
                }
            }
            Element::Record(record) => {
                match self.decode_resource_annotation(cursor, record.annotations()) {
                    Some(Resource::Usage { resource }) => {
                        self.usage_records.insert(
                            record.name().clone(),
                            UsageRecord {
                                resource,
                                claims: record.fields().map(|field| field.name().clone()).collect(),
                                location: cursor.to_string(),
                            },
                        );
                    }
                    Some(Resource::Bounds { resource }) => {
                        self.bounds_records.insert(
                            record.name().clone(),
                            BoundsRecord {
                                resource,
                                fields: record.fields().map(|field| field.name().clone()).collect(),
                                location: cursor.to_string(),
                            },
                        );
                    }
                    Some(role @ Resource::Definition(_)) => {
                        self.errors.push(ResourceError::MisplacedRole {
                            location: cursor.to_string(),
                            role: role.variant_name(),
                            element: "a record",
                        });
                    }
                    None => {}
                }
            }
            Element::Annotations(annotations)
                if !matches!(
                    cursor.previous(),
                    Some(Element::Entity(_) | Element::Record(_))
                ) =>
            {
                if let Some(role) = self.decode_resource_annotation(cursor, annotations) {
                    self.errors.push(ResourceError::MisplacedRole {
                        location: cursor.to_string(),
                        role: role.variant_name(),
                        element: annotation_owner_description(cursor),
                    });
                }
            }
            // Record roles are visited after entity fields, so references are
            // resolved in `finish`.
            Element::DataType(DataType::Record(record)) => {
                self.record_refs.push(RecordRef {
                    record: record.clone(),
                    on_entity_ref: matches!(
                        cursor.previous(),
                        Some(Element::DataType(DataType::EntityRef { .. }))
                    ),
                    entity: enclosing_entity(cursor).map(|entity| {
                        (
                            entity.name().clone(),
                            entity.annotations().has_constraint(FsmConstraint::NAME),
                        )
                    }),
                    location: cursor.to_string(),
                });
            }
            _ => {}
        }
    }

    fn finish(self) -> Self::Output {
        let ResourceConstraint {
            mut errors,
            resources,
            usage_records,
            bounds_records,
            record_refs,
        } = self;

        // Requirements 5 and 6: a usage names a declared resource and claims
        // only that resource's capacities.
        for UsageRecord {
            resource,
            claims,
            location,
        } in usage_records.values()
        {
            let Some(capacities) = resources.get(resource) else {
                errors.push(ResourceError::UnknownResource {
                    location: location.clone(),
                    resource: resource.clone(),
                });
                continue;
            };
            for claim in claims {
                if !capacities.contains_key(claim) {
                    errors.push(ResourceError::UndeclaredCapacity {
                        location: location.clone(),
                        resource: resource.clone(),
                        capacity: claim.clone(),
                    });
                }
            }
        }

        // Requirements 4, 7 and 8: validate references after record roles are
        // known.
        let mut non_fsm_seen = FxHashSet::default();
        let mut resources_with_bounds_event = FxHashSet::default();
        for RecordRef {
            record,
            on_entity_ref,
            entity,
            location,
        } in &record_refs
        {
            if let Some(usage) = usage_records.get(record) {
                // Requirement 7: a usage is carried by an entity reference.
                if !on_entity_ref {
                    errors.push(ResourceError::UsageNotOnReference {
                        location: location.clone(),
                    });
                    continue;
                }
                // Requirement 4: only an FSM entity may use a resource.
                match entity {
                    Some((entity, is_fsm)) => {
                        if !is_fsm && non_fsm_seen.insert((entity.clone(), usage.resource.clone()))
                        {
                            errors.push(ResourceError::NonFsmUser {
                                entity: entity.clone(),
                                resource: usage.resource.clone(),
                            });
                        }
                    }
                    None => errors.push(ResourceError::MisplacedRole {
                        location: location.clone(),
                        role: "usage",
                        element: "a non-entity reference",
                    }),
                }
            } else if let Some(bounds) = bounds_records.get(record) {
                // Requirement 8: a bounds record belongs to its resource, so the
                // entity referencing it must be that resource.
                let on_resource = matches!(entity, Some((entity, _)) if entity == &bounds.resource);
                if !on_resource {
                    errors.push(ResourceError::ForeignBounds {
                        location: location.clone(),
                        resource: bounds.resource.clone(),
                    });
                } else {
                    resources_with_bounds_event.insert(bounds.resource.clone());
                }
            }
        }

        // Requirement 3: a bounds record covers exactly its resource's bounded
        // capacities.
        for BoundsRecord {
            resource,
            fields,
            location,
        } in bounds_records.values()
        {
            let Some(capacities) = resources.get(resource) else {
                errors.push(ResourceError::UnknownResource {
                    location: location.clone(),
                    resource: resource.clone(),
                });
                continue;
            };
            if !capacities.values().any(|bounded| *bounded) {
                errors.push(ResourceError::UnexpectedBounds {
                    location: location.clone(),
                    resource: resource.clone(),
                });
                continue;
            }
            for field in fields {
                if capacities.get(field) != Some(&true) {
                    errors.push(ResourceError::UnboundedCapacity {
                        location: location.clone(),
                        resource: resource.clone(),
                        capacity: field.clone(),
                    });
                }
            }
            for (capacity, bounded) in capacities {
                if *bounded && !fields.contains(capacity) {
                    errors.push(ResourceError::UncoveredCapacity {
                        location: location.clone(),
                        resource: resource.clone(),
                        capacity: capacity.clone(),
                    });
                }
            }
        }

        // Requirement 3: a resource with a bounded capacity has a bounds event.
        for (resource, capacities) in &resources {
            if capacities.values().any(|bounded| *bounded)
                && !resources_with_bounds_event.contains(resource)
            {
                errors.push(ResourceError::MissingBounds {
                    resource: resource.clone(),
                });
            }
        }

        match errors.len() {
            0 => Ok(()),
            1 => Err(errors.into_iter().next().unwrap()),
            _ => Err(ResourceError::Multiple(errors)),
        }
    }
}

impl ResourceConstraint {
    /// Return the [`Resource`] variant attached to `annotations`.
    ///
    /// Record [`ResourceError::InvalidData`] and return `None` for malformed data.
    fn decode_resource_annotation(
        &mut self,
        cursor: &Cursor,
        annotations: &Annotations,
    ) -> Option<Resource> {
        match parse_resource_annotation(annotations) {
            None => None,
            Some(Err(message)) => {
                self.errors.push(ResourceError::InvalidData {
                    location: cursor.to_string(),
                    message,
                });
                None
            }
            Some(Ok(resource)) => Some(resource),
        }
    }

    /// Validate requirements local to a resource definition.
    fn validate_definition(&mut self, resource: &Identifier, capacities: &Capacities) {
        if capacities.is_empty() {
            self.errors.push(ResourceError::NoCapacities {
                resource: resource.clone(),
            });
        }
    }
}

impl Constraint for ResourceConstraint {
    const NAME: &'static str = Resource::NAME;
}

/// Parse the [`Resource`] variant attached to `annotations`.
///
/// Return `None` when no resource constraint is attached and an error when its
/// data is missing or invalid.
fn parse_resource_annotation(annotations: &Annotations) -> Option<Result<Resource, String>> {
    let constraint = annotations.constraint(Resource::NAME)?;
    Some(match constraint.data() {
        None => Err("constraint data is missing".to_string()),
        Some(raw) => serde_json::from_str::<Resource>(raw)
            .map_err(|e| format!("failed to decode resource: {e}")),
    })
}

/// Return the nearest entity enclosing `cursor`.
fn enclosing_entity<'s>(cursor: &Cursor<'s>) -> Option<&'s Entity> {
    cursor
        .elements()
        .iter()
        .rev()
        .find_map(|element| match *element {
            Element::Entity(entity) => Some(entity),
            _ => None,
        })
}

/// Return a diagnostic name for the annotated element.
fn annotation_owner_description(cursor: &Cursor) -> &'static str {
    match cursor.previous() {
        Some(Element::Schema(_)) => "a schema",
        Some(Element::Event(_)) => "an event",
        Some(Element::Field(_)) => "a field",
        Some(Element::DataType(DataType::EntityRef { .. })) => "an entity reference",
        _ => "an unsupported element",
    }
}

/// A resource constraint violation.
#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("{location}: invalid resource data: {message}")]
    InvalidData { location: String, message: String },
    #[error("{location}: a {role} role is misplaced on {element}")]
    MisplacedRole {
        location: String,
        role: &'static str,
        element: &'static str,
    },
    #[error("resource \"{resource}\" declares no capacities")]
    NoCapacities { resource: Identifier },
    #[error("{location}: names undeclared resource \"{resource}\"")]
    UnknownResource {
        location: String,
        resource: Identifier,
    },
    #[error("{location}: claims undeclared capacity \"{capacity}\" of resource \"{resource}\"")]
    UndeclaredCapacity {
        location: String,
        resource: Identifier,
        capacity: Identifier,
    },
    #[error("{location}: a usage record is used outside an entity reference")]
    UsageNotOnReference { location: String },
    #[error("entity \"{entity}\" uses resource \"{resource}\" but is not an FSM")]
    NonFsmUser {
        entity: Identifier,
        resource: Identifier,
    },
    #[error("{location}: bounds of resource \"{resource}\" used outside that resource's events")]
    ForeignBounds {
        location: String,
        resource: Identifier,
    },
    #[error(
        "{location}: bounds declare \"{capacity}\", which resource \"{resource}\" does not bound"
    )]
    UnboundedCapacity {
        location: String,
        resource: Identifier,
        capacity: Identifier,
    },
    #[error("{location}: bounds of resource \"{resource}\" omit bounded capacity \"{capacity}\"")]
    UncoveredCapacity {
        location: String,
        resource: Identifier,
        capacity: Identifier,
    },
    #[error("{location}: unbounded resource \"{resource}\" declares a bounds record")]
    UnexpectedBounds {
        location: String,
        resource: Identifier,
    },
    #[error("resource \"{resource}\" has a bounded capacity but no bounds event")]
    MissingBounds { resource: Identifier },
    #[error("multiple resource violations:\n{}", bullet_list(.0))]
    Multiple(Vec<ResourceError>),
}

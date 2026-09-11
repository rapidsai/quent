// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Constraint for entities representing a Directed Acyclic Graph (DAG).

use std::{fmt::Display, str::FromStr};

use quent_constraints::{Constraint, utils::bullet_list};
use quent_ref_target::RefTarget;
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Identifier, Path,
    builder::AnnotationsBuilder,
    visitor::{Cursor, Element, Visitor},
};
use rustc_hash::{FxHashMap as Map, FxHashSet as Set};
use thiserror::Error;

mod builder;

pub use builder::{DagEntityBuilder, DagEventDecl, EndpointDecl, MembershipDecl};

/// Semantic role of an annotated DAG constituent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DagRole {
    /// An entity representing a DAG.
    Dag,
    /// An entity representing a DAG vertex.
    Vertex,
    /// An entity representing a DAG edge.
    Edge,
    /// A reference declaring DAG membership.
    Membership,
    /// The source vertex reference of an edge.
    Source,
    /// The target vertex reference of an edge.
    Target,
}

impl DagRole {
    /// Constraint identifier.
    pub const NAME: &'static str = "quent.dag.v0.1.0";

    /// Add this role to an annotations builder.
    pub fn annotate(self, annotations: AnnotationsBuilder) -> AnnotationsBuilder {
        annotations.with_constraint(Self::NAME, Some(self.to_string()))
    }

    /// Return annotations carrying this role.
    pub fn annotations(self) -> Annotations {
        self.annotate(AnnotationsBuilder::new())
            .build()
            .expect("the DAG constraint name and role are valid")
    }

    /// Decode a DAG role from `annotations`.
    ///
    /// # Errors
    ///
    /// Returns an error when the constraint has missing or unknown data.
    pub fn from_annotations(annotations: &Annotations) -> Result<Option<Self>, DagRoleParseError> {
        let Some(constraint) = annotations.constraint(Self::NAME) else {
            return Ok(None);
        };
        let raw = constraint.data().ok_or(DagRoleParseError::MissingData)?;
        raw.parse().map(Some)
    }

    fn element_name(self) -> &'static str {
        match self {
            Self::Dag => "dag",
            Self::Vertex => "vertex",
            Self::Edge => "edge",
            Self::Membership => "membership",
            Self::Source => "source",
            Self::Target => "target",
        }
    }
}

impl Display for DagRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.element_name())
    }
}

impl FromStr for DagRole {
    type Err = DagRoleParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dag" => Ok(Self::Dag),
            "vertex" => Ok(Self::Vertex),
            "edge" => Ok(Self::Edge),
            "membership" => Ok(Self::Membership),
            "source" => Ok(Self::Source),
            "target" => Ok(Self::Target),
            _ => Err(DagRoleParseError::UnknownRole(value.to_string())),
        }
    }
}

/// Invalid [`DagRole`] constraint data.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DagRoleParseError {
    #[error("constraint data is missing")]
    MissingData,
    #[error("unknown DAG role {0:?}")]
    UnknownRole(String),
}

/// Validates entity types that form directed acyclic graphs.
///
/// DAGs, vertices, and edges are entities. Vertices and edges declare their DAG
/// membership through a targeted entity reference. Each edge declares exactly
/// one source and target vertex in the same once-event as its membership.
///
/// ## Requirements
///
/// 1. `dag`, `vertex`, and `edge` roles annotate entities.
/// 2. `membership`, `source`, and `target` roles annotate direct event fields.
/// 3. Every vertex and edge has exactly one membership field.
/// 4. A membership field has a targeted entity-reference type.
/// 5. A membership reference points to a DAG type.
/// 6. The event declaring membership has once-cardinality.
/// 7. Every edge has exactly one source and one target field.
/// 8. An edge's membership, source, and target fields occur in the same event.
/// 9. Source and target fields have an entity-reference type.
/// 10. Every source and target reference targets a vertex type.
/// 11. Every endpoint vertex type belongs to the same DAG type as the edge.
///
/// Acyclicity and instance-level reference integrity depend on emitted entity
/// IDs and must be validated when events are reconstructed.
#[derive(Default)]
pub struct DagConstraint {
    errors: Vec<DagError>,
    entities: Set<Path>,
    roles: Map<Path, DagRole>,
    memberships: Map<Path, Vec<Membership>>,
    endpoints: Map<Path, Vec<Endpoint>>,
}

struct Membership {
    event: Identifier,
    cardinality: Cardinality,
    target: Option<Path>,
    valid_type: bool,
    location: String,
}

struct Endpoint {
    role: DagRole,
    event: Identifier,
    cardinality: Cardinality,
    target: Option<Path>,
    valid_type: bool,
    location: String,
}

impl Visitor for DagConstraint {
    type Output = Result<(), DagError>;

    fn visit(&mut self, cursor: &Cursor) {
        match cursor.current() {
            Element::Entity(entity) => {
                self.entities.insert(entity.path().clone());
                match self.decode_role(cursor, entity.annotations()) {
                    Some(role @ (DagRole::Dag | DagRole::Vertex | DagRole::Edge)) => {
                        self.roles.insert(entity.path().clone(), role);
                    }
                    Some(role @ (DagRole::Membership | DagRole::Source | DagRole::Target)) => {
                        self.errors.push(DagError::MisplacedRole {
                            location: cursor.to_string(),
                            role,
                            element: "an entity",
                        });
                    }
                    None => {}
                }
            }
            Element::Field(field) => match self.decode_role(cursor, field.annotations()) {
                Some(role @ DagRole::Membership) => {
                    let Some((entity, event)) = direct_event_at_cursor(cursor) else {
                        self.errors.push(DagError::MisplacedRole {
                            location: cursor.to_string(),
                            role,
                            element: "a non-event field",
                        });
                        return;
                    };
                    let (valid_type, target) = reference_target(field.ty());
                    self.memberships
                        .entry(entity.path().clone())
                        .or_default()
                        .push(Membership {
                            event: event.name().clone(),
                            cardinality: event.cardinality(),
                            target,
                            valid_type,
                            location: cursor.to_string(),
                        });
                }
                Some(role @ (DagRole::Source | DagRole::Target)) => {
                    let Some((entity, event)) = direct_event_at_cursor(cursor) else {
                        self.errors.push(DagError::MisplacedRole {
                            location: cursor.to_string(),
                            role,
                            element: "a non-event field",
                        });
                        return;
                    };
                    let (valid_type, target) = reference_target(field.ty());
                    self.endpoints
                        .entry(entity.path().clone())
                        .or_default()
                        .push(Endpoint {
                            role,
                            event: event.name().clone(),
                            cardinality: event.cardinality(),
                            target,
                            valid_type,
                            location: cursor.to_string(),
                        });
                }
                Some(role @ (DagRole::Dag | DagRole::Vertex | DagRole::Edge)) => {
                    self.errors.push(DagError::MisplacedRole {
                        location: cursor.to_string(),
                        role,
                        element: "a field",
                    });
                }
                None => {}
            },
            Element::Annotations(annotations)
                if !matches!(
                    cursor.previous(),
                    Some(Element::Entity(_) | Element::Field(_))
                ) =>
            {
                if let Some(role) = self.decode_role(cursor, annotations) {
                    self.errors.push(DagError::MisplacedRole {
                        location: cursor.to_string(),
                        role,
                        element: annotation_owner_description(cursor),
                    });
                }
            }
            _ => {}
        }
    }

    fn finish(self) -> Self::Output {
        let DagConstraint {
            mut errors,
            entities,
            roles,
            memberships,
            endpoints,
        } = self;

        let mut dag_parents = Map::default();
        for (entity, role) in &roles {
            if !matches!(role, DagRole::Vertex | DagRole::Edge) {
                continue;
            }
            let declared = memberships
                .get(entity)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let [membership] = declared else {
                match declared {
                    [] => errors.push(DagError::MissingMembership {
                        entity: entity.clone(),
                        role: *role,
                    }),
                    _ => errors.push(DagError::MultipleMemberships {
                        entity: entity.clone(),
                        role: *role,
                        locations: declared
                            .iter()
                            .map(|membership| membership.location.clone())
                            .collect(),
                    }),
                }
                continue;
            };
            if membership.cardinality != Cardinality::Once {
                errors.push(DagError::MembershipEventNotOnce {
                    entity: entity.clone(),
                    event: membership.event.clone(),
                });
            }
            if !membership.valid_type {
                errors.push(DagError::InvalidMembershipType {
                    location: membership.location.clone(),
                });
                continue;
            }
            let Some(target) = &membership.target else {
                errors.push(DagError::UntargetedMembership {
                    location: membership.location.clone(),
                });
                continue;
            };
            if !entities.contains(target) {
                continue;
            }
            if roles.get(target) != Some(&DagRole::Dag) {
                errors.push(DagError::MembershipTargetNotDag {
                    location: membership.location.clone(),
                    target: target.clone(),
                });
                continue;
            }
            dag_parents.insert(entity.clone(), target.clone());
        }

        for (entity, declared) in &memberships {
            if !matches!(roles.get(entity), Some(DagRole::Vertex | DagRole::Edge)) {
                for membership in declared {
                    errors.push(DagError::MembershipOnNonMember {
                        location: membership.location.clone(),
                        entity: entity.clone(),
                    });
                }
            }
        }

        for (entity, declared) in &endpoints {
            if roles.get(entity) != Some(&DagRole::Edge) {
                for endpoint in declared {
                    errors.push(DagError::EndpointOnNonEdge {
                        location: endpoint.location.clone(),
                        role: endpoint.role,
                        entity: entity.clone(),
                    });
                }
            }
        }

        for (edge, role) in &roles {
            if *role != DagRole::Edge {
                continue;
            }
            let declared = endpoints.get(edge).map(Vec::as_slice).unwrap_or_default();
            let sources: Vec<&Endpoint> = declared
                .iter()
                .filter(|endpoint| endpoint.role == DagRole::Source)
                .collect();
            let targets: Vec<&Endpoint> = declared
                .iter()
                .filter(|endpoint| endpoint.role == DagRole::Target)
                .collect();

            check_endpoint_count(edge, DagRole::Source, &sources, &mut errors);
            check_endpoint_count(edge, DagRole::Target, &targets, &mut errors);
            for endpoint in declared {
                if !endpoint.valid_type {
                    errors.push(DagError::InvalidEndpointType {
                        location: endpoint.location.clone(),
                        role: endpoint.role,
                    });
                    continue;
                }
                check_endpoint_target(edge, endpoint, &entities, &roles, &dag_parents, &mut errors);
            }

            let ([source], [target]) = (sources.as_slice(), targets.as_slice()) else {
                continue;
            };
            if source.event != target.event {
                errors.push(DagError::EndpointsInDifferentEvents {
                    edge: edge.clone(),
                    source_event: source.event.clone(),
                    target_event: target.event.clone(),
                });
            } else if source.cardinality != Cardinality::Once {
                errors.push(DagError::EndpointEventNotOnce {
                    edge: edge.clone(),
                    event: source.event.clone(),
                });
            }
            if let Some([membership]) = memberships.get(edge).map(Vec::as_slice)
                && source.event == target.event
                && membership.event != source.event
            {
                errors.push(DagError::MembershipInDifferentEvent {
                    edge: edge.clone(),
                    membership_event: membership.event.clone(),
                    endpoint_event: source.event.clone(),
                });
            }
        }

        match errors.len() {
            0 => Ok(()),
            1 => Err(errors.into_iter().next().unwrap()),
            _ => Err(DagError::Multiple(errors)),
        }
    }
}

impl DagConstraint {
    fn decode_role(&mut self, cursor: &Cursor, annotations: &Annotations) -> Option<DagRole> {
        match DagRole::from_annotations(annotations) {
            Ok(role) => role,
            Err(error) => {
                self.errors.push(DagError::InvalidData {
                    location: cursor.to_string(),
                    message: error.to_string(),
                });
                None
            }
        }
    }
}

impl Constraint for DagConstraint {
    const NAME: &'static str = DagRole::NAME;
}

fn check_endpoint_count(
    edge: &Path,
    role: DagRole,
    endpoints: &[&Endpoint],
    errors: &mut Vec<DagError>,
) {
    match endpoints {
        [] => errors.push(DagError::MissingEndpoint {
            edge: edge.clone(),
            role,
        }),
        [_] => {}
        _ => errors.push(DagError::MultipleEndpoints {
            edge: edge.clone(),
            role,
            locations: endpoints
                .iter()
                .map(|endpoint| endpoint.location.clone())
                .collect(),
        }),
    }
}

fn check_endpoint_target(
    edge: &Path,
    endpoint: &Endpoint,
    entities: &Set<Path>,
    roles: &Map<Path, DagRole>,
    dag_parents: &Map<Path, Path>,
    errors: &mut Vec<DagError>,
) {
    let Some(target) = &endpoint.target else {
        errors.push(DagError::UntargetedEndpoint {
            location: endpoint.location.clone(),
            role: endpoint.role,
        });
        return;
    };
    if !entities.contains(target) {
        return;
    }
    if roles.get(target) != Some(&DagRole::Vertex) {
        errors.push(DagError::EndpointTargetNotVertex {
            location: endpoint.location.clone(),
            role: endpoint.role,
            target: target.clone(),
        });
        return;
    }
    let (Some(edge_dag), Some(vertex_dag)) = (dag_parents.get(edge), dag_parents.get(target))
    else {
        return;
    };
    if edge_dag != vertex_dag {
        errors.push(DagError::EndpointOutsideDag {
            location: endpoint.location.clone(),
            edge: edge.clone(),
            edge_dag: edge_dag.clone(),
            vertex: target.clone(),
            vertex_dag: vertex_dag.clone(),
        });
    }
}

fn reference_target(ty: &DataType) -> (bool, Option<Path>) {
    match ty {
        DataType::EntityRef { annotations, .. } => (
            true,
            RefTarget::from_annotations(annotations).map(Path::from),
        ),
        _ => (false, None),
    }
}

fn direct_event_at_cursor<'s>(cursor: &'s Cursor) -> Option<(&'s Entity, &'s quent_schema::Event)> {
    match cursor.elements() {
        [
            _schema,
            Element::Entity(entity),
            Element::Event(event),
            Element::Field(_),
        ] => Some((entity, event)),
        _ => None,
    }
}

fn annotation_owner_description(cursor: &Cursor) -> &'static str {
    match cursor.previous() {
        Some(Element::Schema(_)) => "a schema",
        Some(Element::Event(_)) => "an event",
        Some(Element::Record(_)) => "a record",
        Some(Element::DataType(_)) => "a data type",
        _ => "an unsupported element",
    }
}

/// A DAG constraint violation.
#[derive(Debug, Error)]
pub enum DagError {
    #[error("{location}: invalid DAG data: {message}")]
    InvalidData { location: String, message: String },
    #[error("{location}: the `{role}` role is misplaced on {element}")]
    MisplacedRole {
        location: String,
        role: DagRole,
        element: &'static str,
    },
    #[error("{role} entity \"{entity}\" has no `membership` reference")]
    MissingMembership { entity: Path, role: DagRole },
    #[error("{role} entity \"{entity}\" has multiple `membership` references: {locations:?}")]
    MultipleMemberships {
        entity: Path,
        role: DagRole,
        locations: Vec<String>,
    },
    #[error(
        "{location}: `membership` is declared by entity \"{entity}\", which is not a vertex or edge"
    )]
    MembershipOnNonMember { location: String, entity: Path },
    #[error("{location}: `membership` must have an entity-reference type")]
    InvalidMembershipType { location: String },
    #[error("{location}: `membership` must have a reference target")]
    UntargetedMembership { location: String },
    #[error("{location}: `membership` targets \"{target}\", which is not a DAG entity")]
    MembershipTargetNotDag { location: String, target: Path },
    #[error("entity \"{entity}\" declares membership in multi-event \"{event}\"")]
    MembershipEventNotOnce { entity: Path, event: Identifier },
    #[error("edge entity \"{edge}\" has no `{role}` reference")]
    MissingEndpoint { edge: Path, role: DagRole },
    #[error("edge entity \"{edge}\" has multiple `{role}` references: {locations:?}")]
    MultipleEndpoints {
        edge: Path,
        role: DagRole,
        locations: Vec<String>,
    },
    #[error("{location}: `{role}` is declared by non-edge entity \"{entity}\"")]
    EndpointOnNonEdge {
        location: String,
        role: DagRole,
        entity: Path,
    },
    #[error("{location}: `{role}` must have an entity-reference type")]
    InvalidEndpointType { location: String, role: DagRole },
    #[error("{location}: `{role}` must have a reference target")]
    UntargetedEndpoint { location: String, role: DagRole },
    #[error(
        "edge entity \"{edge}\" declares its source in event \"{source_event}\" and target in event \"{target_event}\""
    )]
    EndpointsInDifferentEvents {
        edge: Path,
        source_event: Identifier,
        target_event: Identifier,
    },
    #[error("edge entity \"{edge}\" declares endpoints in multi-event \"{event}\"")]
    EndpointEventNotOnce { edge: Path, event: Identifier },
    #[error(
        "edge entity \"{edge}\" declares membership in event \"{membership_event}\" and endpoints in event \"{endpoint_event}\""
    )]
    MembershipInDifferentEvent {
        edge: Path,
        membership_event: Identifier,
        endpoint_event: Identifier,
    },
    #[error("{location}: `{role}` targets \"{target}\", which is not a vertex entity")]
    EndpointTargetNotVertex {
        location: String,
        role: DagRole,
        target: Path,
    },
    #[error(
        "{location}: edge \"{edge}\" belongs to DAG \"{edge_dag}\", but vertex \"{vertex}\" belongs to DAG \"{vertex_dag}\""
    )]
    EndpointOutsideDag {
        location: String,
        edge: Path,
        edge_dag: Path,
        vertex: Path,
        vertex_dag: Path,
    },
    #[error("multiple DAG violations:\n{}", bullet_list(.0))]
    Multiple(Vec<DagError>),
}

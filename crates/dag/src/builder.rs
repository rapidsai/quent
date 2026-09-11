// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_constraints::Constraint;
use quent_ref_target::RefTargetConstraint;
use quent_schema::{
    Annotations, Cardinality, DataType, Entity, Event, Field, Identifier, Path,
    builder::{AnnotationsBuilder, BuilderError, EntityBuilder, EventBuilder},
};

use crate::DagRole;

/// The once-event that declares membership and optional edge endpoints.
pub struct DagEventDecl {
    name: Identifier,
    attributes: Vec<Field>,
    annotations: Annotations,
}

impl DagEventDecl {
    /// Create an empty DAG declaration event.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            attributes: Vec::new(),
            annotations: Annotations::default(),
        }
    }

    /// Add an event field.
    pub fn with_attribute(mut self, attribute: Field) -> Self {
        self.attributes.push(attribute);
        self
    }

    /// Add several event fields.
    pub fn with_attributes(mut self, attributes: impl IntoIterator<Item = Field>) -> Self {
        self.attributes.extend(attributes);
        self
    }

    /// Set the event annotations.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = annotations;
        self
    }
}

/// A vertex or edge reference to its DAG.
pub struct MembershipDecl {
    field: Identifier,
    dag: Path,
    data: Option<Box<DataType>>,
    field_annotations: Annotations,
    reference_annotations: Annotations,
}

impl MembershipDecl {
    /// Create a membership reference named `field` targeting `dag`.
    pub fn new(field: Identifier, dag: impl Into<Path>) -> Self {
        Self {
            field,
            dag: dag.into(),
            data: None,
            field_annotations: Annotations::default(),
            reference_annotations: Annotations::default(),
        }
    }

    /// Set data carried by the entity reference.
    pub fn with_data(mut self, data: DataType) -> Self {
        self.data = Some(Box::new(data));
        self
    }

    /// Set annotations on the membership field.
    pub fn with_field_annotations(mut self, annotations: Annotations) -> Self {
        self.field_annotations = annotations;
        self
    }

    /// Set annotations on the entity-reference type.
    pub fn with_reference_annotations(mut self, annotations: Annotations) -> Self {
        self.reference_annotations = annotations;
        self
    }

    fn build(self) -> Result<Field, BuilderError> {
        reference_field(
            self.field,
            Some(self.dag),
            self.data,
            self.field_annotations,
            self.reference_annotations,
            DagRole::Membership,
        )
    }
}

/// A source or target reference declared by an edge.
pub struct EndpointDecl {
    field: Identifier,
    vertex: Path,
    data: Option<Box<DataType>>,
    field_annotations: Annotations,
    reference_annotations: Annotations,
}

impl EndpointDecl {
    /// Create an endpoint reference named `field` targeting `vertex`.
    pub fn new(field: Identifier, vertex: impl Into<Path>) -> Self {
        Self {
            field,
            vertex: vertex.into(),
            data: None,
            field_annotations: Annotations::default(),
            reference_annotations: Annotations::default(),
        }
    }

    /// Set data carried by the entity reference.
    pub fn with_data(mut self, data: DataType) -> Self {
        self.data = Some(Box::new(data));
        self
    }

    /// Set annotations on the endpoint field.
    pub fn with_field_annotations(mut self, annotations: Annotations) -> Self {
        self.field_annotations = annotations;
        self
    }

    /// Set annotations on the entity-reference type.
    pub fn with_reference_annotations(mut self, annotations: Annotations) -> Self {
        self.reference_annotations = annotations;
        self
    }

    fn build(self, role: DagRole) -> Result<Field, BuilderError> {
        reference_field(
            self.field,
            Some(self.vertex),
            self.data,
            self.field_annotations,
            self.reference_annotations,
            role,
        )
    }
}

enum Declaration {
    None,
    Vertex {
        event: DagEventDecl,
        membership: MembershipDecl,
    },
    Edge {
        event: DagEventDecl,
        membership: MembershipDecl,
        source: Box<EndpointDecl>,
        target: Box<EndpointDecl>,
    },
}

/// Builds a DAG, vertex, or edge entity.
///
/// Vertex and edge constructors generate the required once-event and topology
/// fields. Reference targets are validated with the containing schema.
pub struct DagEntityBuilder {
    path: Path,
    events: Vec<Event>,
    annotations: Annotations,
    role: DagRole,
    declaration: Declaration,
}

impl DagEntityBuilder {
    /// Start a DAG entity at `path`.
    pub fn dag(path: impl Into<Path>) -> Self {
        Self {
            path: path.into(),
            events: Vec::new(),
            annotations: Annotations::default(),
            role: DagRole::Dag,
            declaration: Declaration::None,
        }
    }

    /// Start a vertex entity.
    pub fn vertex(path: impl Into<Path>, event: DagEventDecl, membership: MembershipDecl) -> Self {
        Self {
            path: path.into(),
            events: Vec::new(),
            annotations: Annotations::default(),
            role: DagRole::Vertex,
            declaration: Declaration::Vertex { event, membership },
        }
    }

    /// Start an edge entity.
    pub fn edge(
        path: impl Into<Path>,
        event: DagEventDecl,
        membership: MembershipDecl,
        source: EndpointDecl,
        target: EndpointDecl,
    ) -> Self {
        Self {
            path: path.into(),
            events: Vec::new(),
            annotations: Annotations::default(),
            role: DagRole::Edge,
            declaration: Declaration::Edge {
                event,
                membership,
                source: Box::new(source),
                target: Box::new(target),
            },
        }
    }

    /// Set entity annotations.
    ///
    /// The DAG role is added on [`Self::build`].
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = annotations;
        self
    }

    /// Add an entity event.
    pub fn with_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }

    /// Add several entity events.
    pub fn with_events(mut self, events: impl IntoIterator<Item = Event>) -> Self {
        self.events.extend(events);
        self
    }

    /// Build the entity with its DAG annotations.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicate event or field names, or when a DAG
    /// entity declares no events.
    pub fn build(self) -> Result<Entity, BuilderError> {
        let declaration = match self.declaration {
            Declaration::None => None,
            Declaration::Vertex { event, membership } => {
                Some(build_declaration_event(event, [membership.build()?])?)
            }
            Declaration::Edge {
                event,
                membership,
                source,
                target,
            } => Some(build_declaration_event(
                event,
                [
                    membership.build()?,
                    (*source).build(DagRole::Source)?,
                    (*target).build(DagRole::Target)?,
                ],
            )?),
        };
        let mut events = self.events;
        if let Some(event) = declaration {
            events.push(event);
        }
        let annotations = AnnotationsBuilder::from_annotations(&self.annotations)
            .with_constraint(DagRole::NAME, Some(self.role.to_string()))
            .build()?;
        EntityBuilder::new(self.path)
            .with_events(events)
            .with_annotations(annotations)
            .build()
    }
}

fn build_declaration_event(
    declaration: DagEventDecl,
    topology: impl IntoIterator<Item = Field>,
) -> Result<Event, BuilderError> {
    EventBuilder::new(declaration.name, Cardinality::Once)
        .with_annotations(declaration.annotations)
        .with_fields(topology.into_iter().chain(declaration.attributes))
        .build()
}

fn reference_field(
    name: Identifier,
    target: Option<Path>,
    data: Option<Box<DataType>>,
    field_annotations: Annotations,
    reference_annotations: Annotations,
    role: DagRole,
) -> Result<Field, BuilderError> {
    let field_annotations = AnnotationsBuilder::from_annotations(&field_annotations)
        .with_constraint(DagRole::NAME, Some(role.to_string()))
        .build()?;

    let mut reference_annotations = AnnotationsBuilder::from_annotations(&reference_annotations);
    if let Some(target) = target {
        reference_annotations = reference_annotations
            .with_constraint(RefTargetConstraint::NAME, Some(target.to_string()));
    }
    let reference_annotations = reference_annotations.build()?;

    Ok(Field::new(
        name,
        DataType::EntityRef {
            data,
            annotations: reference_annotations,
        },
        field_annotations,
    ))
}

// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{
    Annotations, DataType, Field, Identifier, Record,
    builder::{AnnotationsBuilder, BuilderError, RecordBuilder},
    schema::identifier::IdentifierError,
};
use thiserror::Error;

use crate::{Capacities, Capacity, Resource};

/// The artifacts a [`ResourceBuilder`] delivers for a resource.
pub struct ResourceParts {
    /// The resource constraint definition to place on the resource entity's
    /// constraints.
    pub definition: Resource,
    /// The record type a usage of the resource is carried as.
    pub usage: Record,
    /// The record type the resource's bounds are carried as, present iff a
    /// capacity is bounded.
    pub bounds: Option<Record>,
}

/// Builds a resource definition and its usage and bounds record types.
pub struct ResourceBuilder {
    name: Identifier,
    capacities: Capacities,
    errors: Vec<BuildError>,
}

impl ResourceBuilder {
    /// Start a resource named `name`.
    pub fn new(name: Identifier) -> Self {
        Self {
            name,
            capacities: Capacities::default(),
            errors: Vec::new(),
        }
    }

    /// The capacity declared under `name`, if any.
    pub fn capacity(&self, name: &Identifier) -> Option<&Capacity> {
        self.capacities.get(name)
    }

    /// Set a capacity, returning the replaced declaration, if any.
    pub fn set_capacity(&mut self, name: Identifier, capacity: Capacity) -> Option<Capacity> {
        self.capacities.insert(name, capacity)
    }

    /// Add a capacity, returning the builder for chaining.
    pub fn with_capacity(mut self, name: Identifier, capacity: Capacity) -> Self {
        if self.capacities.contains_key(&name) {
            self.errors.push(BuildError::DuplicateCapacity(name));
        } else {
            self.capacities.insert(name, capacity);
        }
        self
    }

    /// Add several capacities, returning the builder for chaining.
    pub fn with_capacities(
        mut self,
        capacities: impl IntoIterator<Item = (Identifier, Capacity)>,
    ) -> Self {
        for (name, capacity) in capacities {
            self = self.with_capacity(name, capacity);
        }
        self
    }

    /// Build the definition and the usage and bounds record types.
    ///
    /// # Errors
    ///
    /// Errors if no capacity was added, a name is repeated, or generating the
    /// records or constraint data fails.
    pub fn build(self) -> Result<ResourceParts, BuildError> {
        let ResourceBuilder {
            name,
            capacities,
            mut errors,
        } = self;
        if capacities.is_empty() {
            errors.push(BuildError::NoCapacities);
        }
        match errors.len() {
            0 => {}
            1 => return Err(errors.pop().unwrap()),
            _ => return Err(BuildError::Multiple(errors)),
        }

        // The usage record carries a claim field for each capacity.
        let usage = build_resource_record(
            suffixed_identifier(&name, "Usage")?,
            Resource::Usage {
                resource: name.clone(),
            },
            capacities.keys(),
        )?;

        // The bounds record carries a field for each bounded capacity, if any.
        let bounds = if capacities.values().any(Capacity::is_bounded) {
            Some(build_resource_record(
                suffixed_identifier(&name, "Bounds")?,
                Resource::Bounds {
                    resource: name.clone(),
                },
                capacities
                    .iter()
                    .filter(|(_, capacity)| capacity.is_bounded())
                    .map(|(name, _)| name),
            )?)
        } else {
            None
        };

        Ok(ResourceParts {
            definition: Resource::Definition(capacities),
            usage,
            bounds,
        })
    }
}

/// Build a record carrying a resource annotation.
///
/// Each supplied identifier becomes a `U64` field.
fn build_resource_record<'a>(
    name: Identifier,
    resource: Resource,
    fields: impl Iterator<Item = &'a Identifier>,
) -> Result<Record, BuildError> {
    let annotations = AnnotationsBuilder::new()
        .try_with_constraint(Resource::NAME, Some(serde_json::to_string(&resource)?))?
        .build();
    let mut builder = RecordBuilder::new(name).with_annotations(annotations);
    for field in fields {
        builder = builder.try_with_field(Field::new(
            field.clone(),
            DataType::U64,
            Annotations::default(),
        ))?;
    }
    Ok(builder.build())
}

fn suffixed_identifier(resource: &Identifier, suffix: &str) -> Result<Identifier, BuildError> {
    Ok(Identifier::try_new(format!("{resource}{suffix}"))?)
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("resource must declare at least one capacity")]
    NoCapacities,
    #[error("duplicate capacity \"{0}\"")]
    DuplicateCapacity(Identifier),
    #[error("multiple resource builder errors: {0:?}")]
    Multiple(Vec<BuildError>),
    #[error(transparent)]
    Schema(#[from] BuilderError),
    #[error(transparent)]
    Identifier(#[from] IdentifierError),
    #[error("serializing resource data: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapacityKind;

    #[test]
    fn builds_definition_and_records() -> Result<(), BuildError> {
        let bytes = Identifier::try_new("bytes")?;
        let usage_name = Identifier::try_new("MemoryUsage")?;
        let bounds_name = Identifier::try_new("MemoryBounds")?;

        let parts = ResourceBuilder::new(Identifier::try_new("Memory")?)
            .with_capacity(bytes.clone(), Capacity::new(CapacityKind::Occupancy, true))
            .build()?;

        let mut capacities = parts.definition.capacities().unwrap();
        let (name, capacity) = capacities.next().unwrap();
        assert_eq!(name, &bytes);
        assert_eq!(capacity.kind(), CapacityKind::Occupancy);
        assert!(capacity.is_bounded());
        assert!(capacities.next().is_none());
        assert_eq!(parts.usage.name(), &usage_name);
        assert!(parts.usage.field(&bytes).is_some());
        assert!(
            parts
                .bounds
                .is_some_and(|bounds| bounds.name() == &bounds_name)
        );
        Ok(())
    }

    #[test]
    fn rejects_empty_resource() {
        let result = ResourceBuilder::new(Identifier::try_new("Memory").unwrap()).build();
        assert!(matches!(result, Err(BuildError::NoCapacities)));
    }

    /// Requirement 2: capacity identifiers are unique within a resource.
    #[test]
    fn rejects_duplicate_capacities() {
        let bytes = Identifier::try_new("bytes").unwrap();
        let watts = Identifier::try_new("watts").unwrap();
        let result = ResourceBuilder::new(Identifier::try_new("Memory").unwrap())
            .with_capacity(bytes.clone(), Capacity::new(CapacityKind::Occupancy, true))
            .with_capacity(bytes.clone(), Capacity::new(CapacityKind::Rate, false))
            .with_capacity(watts.clone(), Capacity::new(CapacityKind::Rate, false))
            .with_capacity(watts, Capacity::new(CapacityKind::Rate, true))
            .build();
        let Err(BuildError::Multiple(errors)) = result else {
            panic!("expected multiple errors");
        };
        assert_eq!(errors.len(), 2);
        assert!(matches!(&errors[0], BuildError::DuplicateCapacity(name) if name == &bytes));
        assert!(matches!(&errors[1], BuildError::DuplicateCapacity(_)));
    }
}

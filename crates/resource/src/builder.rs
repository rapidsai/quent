// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_schema::{
    Annotations, DataType, Field, Identifier, Path, Record,
    builder::{AnnotationsBuilder, BuilderError, RecordBuilder},
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
    path: Path,
    usage_record_path: Path,
    bounds_record_path: Path,
    capacities: Vec<(Identifier, Capacity)>,
}

impl ResourceBuilder {
    /// Start a resource using sibling `{name}Usage` and `{name}Bounds` record paths.
    pub fn new(path: impl Into<Path>) -> Self {
        let path = path.into();
        let usage_record_path = Self::default_usage_record_path(&path);
        let bounds_record_path = Self::default_bounds_record_path(&path);
        Self::with_record_paths(path, usage_record_path, bounds_record_path)
    }

    /// Return the default usage record path for `resource`.
    pub fn default_usage_record_path(resource: &Path) -> Path {
        resource.with_name(suffixed_identifier(resource.name(), "Usage"))
    }

    /// Return the default bounds record path for `resource`.
    pub fn default_bounds_record_path(resource: &Path) -> Path {
        resource.with_name(suffixed_identifier(resource.name(), "Bounds"))
    }

    /// Start a resource with explicit generated record paths.
    ///
    /// `bounds_record_path` is used only when a capacity is bounded.
    pub fn with_record_paths(
        path: impl Into<Path>,
        usage_record_path: impl Into<Path>,
        bounds_record_path: impl Into<Path>,
    ) -> Self {
        Self {
            path: path.into(),
            usage_record_path: usage_record_path.into(),
            bounds_record_path: bounds_record_path.into(),
            capacities: Vec::new(),
        }
    }

    /// Start a resource with explicit generated record paths.
    pub fn with_record_names(
        path: impl Into<Path>,
        usage_record_path: impl Into<Path>,
        bounds_record_path: impl Into<Path>,
    ) -> Self {
        Self::with_record_paths(path, usage_record_path, bounds_record_path)
    }

    /// Add a capacity, returning the builder for chaining.
    pub fn with_capacity(mut self, name: Identifier, capacity: Capacity) -> Self {
        self.capacities.push((name, capacity));
        self
    }

    /// Add several capacities, returning the builder for chaining.
    pub fn with_capacities(
        mut self,
        capacities: impl IntoIterator<Item = (Identifier, Capacity)>,
    ) -> Self {
        self.capacities.extend(capacities);
        self
    }

    /// Build the definition and its usage and bounds record types.
    ///
    /// With no capacities the result is a unit resource: an empty usage record
    /// and no bounds.
    ///
    /// # Errors
    ///
    /// Errors if a capacity name is repeated, the supplied names are equal when
    /// bounds are generated, or generation fails.
    pub fn build(self) -> Result<ResourceParts, BuildError> {
        let ResourceBuilder {
            path,
            usage_record_path,
            bounds_record_path,
            capacities,
        } = self;
        let mut unique_capacities = Capacities::default();
        for (name, capacity) in capacities {
            match unique_capacities.entry(name) {
                indexmap::map::Entry::Occupied(entry) => {
                    return Err(BuildError::DuplicateCapacity(entry.key().clone()));
                }
                indexmap::map::Entry::Vacant(entry) => {
                    entry.insert(capacity);
                }
            }
        }
        let capacities = unique_capacities;
        let has_bounds = capacities.values().any(Capacity::is_bounded);
        if has_bounds && usage_record_path == bounds_record_path {
            return Err(BuildError::DuplicateRecordName(usage_record_path));
        }

        // Usages include every capacity. Bounds omit unbounded capacities
        // because those have no bound value to carry.
        let usage = build_resource_record(
            usage_record_path,
            Resource::Usage {
                resource: path.clone(),
            },
            capacities.keys(),
        )?;

        let bounds = if has_bounds {
            Some(build_resource_record(
                bounds_record_path,
                Resource::Bounds {
                    resource: path.clone(),
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
    path: Path,
    resource: Resource,
    fields: impl Iterator<Item = &'a Identifier>,
) -> Result<Record, BuildError> {
    let annotations = AnnotationsBuilder::new()
        .with_constraint(Resource::NAME, Some(resource.constraint_data()?))
        .build()?;
    let mut builder = RecordBuilder::new(path).with_annotations(annotations);
    for field in fields {
        builder = builder.with_field(Field::new(
            field.clone(),
            DataType::U64,
            Annotations::default(),
        ));
    }
    Ok(builder.build()?)
}

fn suffixed_identifier(resource: &Identifier, suffix: &str) -> Identifier {
    Identifier::try_new(format!("{resource}{suffix}"))
        .expect("suffixing a valid identifier preserves validity")
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("duplicate capacity \"{0}\"")]
    DuplicateCapacity(Identifier),
    #[error("usage and bounds records have the same path \"{0}\"")]
    DuplicateRecordName(Path),
    #[error(transparent)]
    Schema(#[from] BuilderError),
    #[error("serializing resource data: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapacityKind;
    use quent_schema::test_utils::{ident, path};

    #[test]
    fn builds_definition_and_records() {
        let bytes = ident("bytes");
        let parts = ResourceBuilder::new(ident("Memory"))
            .with_capacity(bytes.clone(), Capacity::new(CapacityKind::Occupancy, true))
            .build()
            .unwrap();

        let mut capacities = parts.definition.capacities().unwrap();
        let (name, capacity) = capacities.next().unwrap();
        assert_eq!(name, &bytes);
        assert_eq!(capacity.kind(), CapacityKind::Occupancy);
        assert!(capacity.is_bounded());
        assert!(capacities.next().is_none());
        assert_eq!(parts.usage.path(), &path("MemoryUsage"));
        assert!(parts.usage.field(&bytes).is_some());
        assert!(
            parts
                .bounds
                .is_some_and(|bounds| bounds.path() == &path("MemoryBounds"))
        );
    }

    /// A resource with no capacities is a unit resource: a fieldless usage
    /// record and no bounds.
    #[test]
    fn builds_unit_resource() {
        let parts = ResourceBuilder::new(ident("Thread")).build().unwrap();
        assert!(parts.definition.capacities().unwrap().next().is_none());
        assert_eq!(parts.usage.path(), &path("ThreadUsage"));
        assert_eq!(parts.usage.fields().count(), 0);
        assert!(parts.bounds.is_none());
    }

    #[test]
    fn uses_supplied_record_names() {
        let parts = ResourceBuilder::with_record_names(
            ident("Memory"),
            ident("MemoryClaim"),
            ident("MemoryLimits"),
        )
        .with_capacity(ident("bytes"), Capacity::new(CapacityKind::Occupancy, true))
        .build()
        .unwrap();

        assert_eq!(parts.usage.path(), &path("MemoryClaim"));
        assert_eq!(parts.bounds.unwrap().path(), &path("MemoryLimits"));
    }

    #[test]
    fn rejects_duplicate_record_names() {
        let shared = ident("MemoryData");
        let result = ResourceBuilder::with_record_names(ident("Memory"), shared.clone(), shared)
            .with_capacity(ident("bytes"), Capacity::new(CapacityKind::Occupancy, true))
            .build();
        assert!(matches!(result, Err(BuildError::DuplicateRecordName(_))));
    }

    /// Requirement 1: capacity identifiers are unique within a resource.
    #[test]
    fn rejects_duplicate_capacities() {
        let bytes = ident("bytes");
        let watts = ident("watts");
        let result = ResourceBuilder::new(ident("Memory"))
            .with_capacity(bytes.clone(), Capacity::new(CapacityKind::Occupancy, true))
            .with_capacity(bytes.clone(), Capacity::new(CapacityKind::Rate, false))
            .with_capacity(watts.clone(), Capacity::new(CapacityKind::Rate, false))
            .with_capacity(watts, Capacity::new(CapacityKind::Rate, true))
            .build();
        assert!(matches!(
            result,
            Err(BuildError::DuplicateCapacity(name)) if name == bytes
        ));
    }

    #[test]
    fn generated_records_are_siblings_of_the_resource() {
        let parts = ResourceBuilder::new(path("Foo::Memory"))
            .with_capacity(ident("bytes"), Capacity::new(CapacityKind::Occupancy, true))
            .build()
            .unwrap();

        assert_eq!(parts.usage.path().to_string(), "Foo::MemoryUsage");
        assert_eq!(
            parts.bounds.unwrap().path().to_string(),
            "Foo::MemoryBounds"
        );
    }
}

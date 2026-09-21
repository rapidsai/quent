// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Collections of schema-defined resources.

use uuid::Uuid;

use crate::{
    AnalyzerResult,
    resource::{Resource, ResourceTypeDecl},
};

/// Trait for types holding a collection of [`Resource`]s.
pub trait ResourceCollection {
    /// Return an iterator over all `Resource`s in this collection.
    fn resources(&self) -> impl Iterator<Item = &dyn Resource>;

    /// Return a reference to the [`Resource`] with the provided ID.
    fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource>;

    /// Return the [`ResourceTypeDecl`] of the resource type with the provided
    /// resource type name.
    fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl>;

    /// Return the [`ResourceTypeDecl`] of the resource type with the provided
    /// resource .
    fn resource_type_of(&self, resource_id: Uuid) -> AnalyzerResult<&ResourceTypeDecl> {
        self.resource_type(self.resource(resource_id)?.type_name())
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use quent_time::TimeUnixNanoSec;
    use rustc_hash::FxHashMap as HashMap;

    use super::*;
    use crate::{AnalyzerError, Entity};

    pub(crate) struct TestResource {
        id: Uuid,
        type_name: String,
    }

    impl Entity for TestResource {
        fn id(&self) -> Uuid {
            self.id
        }

        fn type_name(&self) -> &str {
            &self.type_name
        }

        fn earliest_timestamp(&self) -> TimeUnixNanoSec {
            0
        }

        fn latest_timestamp(&self) -> TimeUnixNanoSec {
            0
        }
    }

    impl Resource for TestResource {}

    #[derive(Default)]
    pub(crate) struct TestResources {
        resource_types: HashMap<String, ResourceTypeDecl>,
        resources: HashMap<Uuid, TestResource>,
    }

    impl TestResources {
        pub(crate) fn insert_type(&mut self, declaration: ResourceTypeDecl) {
            self.resource_types
                .insert(declaration.name.clone(), declaration);
        }

        pub(crate) fn insert_resource(&mut self, id: Uuid, type_name: impl Into<String>) {
            self.resources.insert(
                id,
                TestResource {
                    id,
                    type_name: type_name.into(),
                },
            );
        }
    }

    impl ResourceCollection for TestResources {
        fn resources(&self) -> impl Iterator<Item = &dyn Resource> {
            self.resources
                .values()
                .map(|resource| resource as &dyn Resource)
        }

        fn resource(&self, resource_id: Uuid) -> AnalyzerResult<&dyn Resource> {
            self.resources
                .get(&resource_id)
                .map(|resource| resource as &dyn Resource)
                .ok_or(AnalyzerError::InvalidId(resource_id))
        }

        fn resource_type(&self, resource_type_name: &str) -> AnalyzerResult<&ResourceTypeDecl> {
            self.resource_types
                .get(resource_type_name)
                .ok_or_else(|| AnalyzerError::InvalidTypeName(resource_type_name.to_owned()))
        }
    }
}

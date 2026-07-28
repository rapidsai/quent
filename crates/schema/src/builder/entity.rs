// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::builder::{AnnotationsBuilder, BuilderError, collect_unique};
use crate::{Annotations, Entity, Event, Path, PathError};

/// Builder for an [`Entity`].
pub struct EntityBuilder {
    path: Path,
    events: Vec<Event>,
    annotations: AnnotationsBuilder,
}

impl EntityBuilder {
    /// Starts an entity whose identity is the supplied qualified path.
    pub fn new(path: impl Into<Path>) -> Self {
        Self {
            path: path.into(),
            events: Vec::new(),
            annotations: AnnotationsBuilder::new(),
        }
    }

    /// Start an entity at `path`, validating its segments.
    ///
    /// # Errors
    ///
    /// Errors if `path` is not a valid path.
    pub fn try_new(path: impl AsRef<str>) -> Result<Self, PathError> {
        Ok(Self::new(path.as_ref().parse::<Path>()?))
    }

    /// Add an event, returning the builder for chaining.
    pub fn with_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }

    /// Add several events, returning the builder for chaining.
    pub fn with_events(mut self, events: impl IntoIterator<Item = Event>) -> Self {
        self.events.extend(events);
        self
    }

    /// Set the entity's annotations, replacing any added so far, and return
    /// the builder for chaining.
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = AnnotationsBuilder::from_annotations(&annotations);
        self
    }

    /// Finish building the entity.
    ///
    /// # Errors
    ///
    /// Errors if the entity declares no events, an event name is repeated, or
    /// the annotations are invalid.
    pub fn build(self) -> Result<Entity, BuilderError> {
        let Self {
            path,
            events,
            annotations,
        } = self;
        if events.is_empty() {
            return Err(BuilderError::NoEvents);
        }
        let events = collect_unique(events, |event| event.name().clone())?;
        let annotations = annotations.build()?;
        Ok(Entity::from_parts(path, events, annotations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_entity_without_events() {
        let error = EntityBuilder::try_new("E").unwrap().build().unwrap_err();
        assert_eq!(error, BuilderError::NoEvents);
    }
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Ordered, producer-defined information shown for query-engine entities.

use quent_model::{Attributes, attributes::DynamicAttributes};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// An invalid producer-defined information group.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum InformationGroupError {
    #[error("information group heading cannot be empty")]
    EmptyHeading,
    #[error("information group '{0}' cannot be empty")]
    EmptyItems(String),
}

/// A producer-defined heading and its information items, in display order.
#[derive(Debug, Attributes, Deserialize, Serialize)]
pub struct InformationGroup {
    pub heading: String,
    pub items: DynamicAttributes,
}

impl InformationGroup {
    /// Creates a validated producer-defined information group.
    pub fn try_new(
        heading: impl Into<String>,
        items: DynamicAttributes,
    ) -> Result<Self, InformationGroupError> {
        let heading = heading.into();
        if heading.trim().is_empty() {
            return Err(InformationGroupError::EmptyHeading);
        }
        if items.is_empty() {
            return Err(InformationGroupError::EmptyItems(heading));
        }
        Ok(Self { heading, items })
    }

    pub fn heading(&self) -> &str {
        &self.heading
    }

    pub fn items(&self) -> &DynamicAttributes {
        &self.items
    }
}

/// Ordered producer-defined information groups.
pub type InformationGroups = Vec<InformationGroup>;

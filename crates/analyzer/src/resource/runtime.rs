// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Run-time defined Resources and Resource Groups (in analysis)

use quent_attributes::Attribute;
use quent_time::{TimeOrderedCollector, TimeUnixNanoSec, Timestamp};
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, Transition},
    resource::{Resource, ResourceCapacities, ResourceGroup},
};

/// Resource state transitions.
pub enum RtResourceTransition {
    Init(TimeUnixNanoSec),
    Operating(TimeUnixNanoSec, ResourceCapacities),
    Resizing(TimeUnixNanoSec),
    Finalizing(TimeUnixNanoSec),
    Exit(TimeUnixNanoSec),
}

impl Timestamp for RtResourceTransition {
    fn timestamp(&self) -> TimeUnixNanoSec {
        *match self {
            RtResourceTransition::Init(ts) => ts,
            RtResourceTransition::Operating(ts, _) => ts,
            RtResourceTransition::Resizing(ts) => ts,
            RtResourceTransition::Finalizing(ts) => ts,
            RtResourceTransition::Exit(ts) => ts,
        }
    }
}

impl Transition for RtResourceTransition {
    fn name(&self) -> &str {
        match self {
            RtResourceTransition::Init(_) => "init",
            RtResourceTransition::Operating(_, _) => "operating",
            RtResourceTransition::Resizing(_) => "resizing",
            RtResourceTransition::Finalizing(_) => "finalizing",
            RtResourceTransition::Exit(_) => "exit",
        }
    }

    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        std::iter::empty()
    }
}

pub struct RtResourceBuilder {
    id: Uuid,
    instance_name: Option<String>,
    type_name: Option<String>,
    parent_group_id: Option<Uuid>,
    transitions: TimeOrderedCollector<RtResourceTransition>,
}

impl RtResourceBuilder {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::InvalidId(id))
        } else {
            Ok(Self {
                id,
                instance_name: None,
                type_name: None,
                parent_group_id: None,
                transitions: Default::default(),
            })
        }
    }
    pub fn set_type_name(&mut self, type_name: impl Into<String>) {
        self.type_name = Some(type_name.into());
    }
    pub fn set_instance_name(&mut self, instance_name: Option<String>) {
        self.instance_name = instance_name;
    }
    pub fn set_parent_group_id(&mut self, parent: Uuid) {
        self.parent_group_id = Some(parent);
    }
    pub fn push(&mut self, transition: RtResourceTransition) {
        self.transitions.push(transition);
    }
    pub fn try_build(self) -> AnalyzerResult<RtResource> {
        let transitions: Vec<RtResourceTransition> = self.transitions.into_inner();

        if transitions.len() < 4 {
            return Err(AnalyzerError::Validation(format!(
                "resource {} expected to have at least 4 transitions (init, operating, finalizing, exit), has {} instead",
                self.id,
                transitions.len()
            )));
        }
        if !matches!(transitions.first().unwrap(), RtResourceTransition::Init(_),) {
            return Err(AnalyzerError::Validation(format!(
                "last state of resource {} is not exit",
                self.id
            )));
        }
        if !matches!(transitions.last().unwrap(), RtResourceTransition::Exit(_),) {
            return Err(AnalyzerError::Validation(format!(
                "last state of resource {} is not exit",
                self.id
            )));
        }

        // TODO(johanpel): validate more transition logic

        Ok(RtResource {
            id: self.id,
            instance_name: self.instance_name.ok_or_else(|| {
                AnalyzerError::IncompleteEntity(format!(
                    "resource {} must have an instance name",
                    self.id
                ))
            })?,
            type_name: self.type_name.ok_or_else(|| {
                AnalyzerError::IncompleteEntity(format!(
                    "resource {} must have a type name",
                    self.id
                ))
            })?,
            parent_group_id: self.parent_group_id.ok_or_else(|| {
                AnalyzerError::IncompleteEntity(format!(
                    "resource {} must have a parent resource group",
                    self.id
                ))
            })?,
            transitions,
        })
    }
}

/// A Resource.
pub struct RtResource {
    /// The ID of this Resource.
    pub id: Uuid,
    /// The name of this Resource.
    pub instance_name: String,
    /// The unique type name of this Resource.
    pub type_name: String,
    /// The id of the parent Resource Group.
    pub parent_group_id: Uuid,
    /// The sequence of state transitions this resource went through.
    pub transitions: Vec<RtResourceTransition>,
}

impl Entity for RtResource {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        self.type_name.as_str()
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_str()
    }
}

impl Fsm for RtResource {
    type TransitionType = RtResourceTransition;
    fn len(&self) -> usize {
        self.transitions.len() - 1 // -1 for the exit transition.
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.transitions.get(index)
    }
}

impl Resource for RtResource {
    fn parent_group_id(&self) -> Uuid {
        self.parent_group_id
    }
}

/// A Group of [`Resource`]s.
#[derive(Clone, Debug, Default)]
pub struct RtResourceGroup {
    /// The ID of this Resource Group.
    pub id: Uuid,
    /// The name of the type of Resource Group
    pub type_name: String,
    /// The name of the instance of this Resource Group.
    pub instance_name: String,
    /// The parent of this Resource Group.
    ///
    /// If this is None, it is considered the root of the global application's
    /// resource tree.
    pub parent_group_id: Option<Uuid>,
}

impl RtResourceGroup {
    pub fn try_new(
        id: Uuid,
        type_name: String,
        instance_name: String,
        parent_group_id: Option<Uuid>,
    ) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::InvalidId(id))
        } else {
            Ok(Self {
                id,
                type_name,
                instance_name,
                parent_group_id,
            })
        }
    }
}

impl Entity for RtResourceGroup {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        self.type_name.as_str()
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_str()
    }
}

impl ResourceGroup for RtResourceGroup {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.parent_group_id
    }
}

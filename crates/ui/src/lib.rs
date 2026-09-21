// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_analyzer::{self as a, AnalyzerResult, Entity, Model, resource::tree::ResourceTreeNode};
use quent_dynamic_attributes::DynamicAttribute;
use quent_time::{TimeSec, TimeUnixNanoSec, try_to_secs_relative};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

pub mod entities;
pub mod fsm;
pub mod paginate;
pub mod quantity;
pub mod timeline;

/// A type of [`Resource`].
#[derive(TS, Serialize, Clone, Debug, Default)]
pub struct ResourceTypeDecl {
    /// The unique type name for this type of Resource.
    pub name: String,
    /// The capacities of this type of Resource.
    pub capacities: Vec<quantity::CapacityDecl>,
    /// The type names of the entities that used this Resource.
    pub used_by: Vec<String>,
}

impl From<&a::resource::ResourceTypeDecl> for ResourceTypeDecl {
    fn from(value: &a::resource::ResourceTypeDecl) -> Self {
        Self {
            name: value.name.clone(),
            capacities: value
                .capacities
                .iter()
                .map(|cap| quantity::CapacityDecl {
                    name: cap.name.clone(),
                    kind: match cap.kind {
                        a::resource::CapacityType::Occupancy => quantity::CapacityKind::Occupancy,
                        a::resource::CapacityType::Rate => quantity::CapacityKind::Rate,
                    },
                    quantity: cap.name.clone(),
                })
                .collect(),
            used_by: value.used_by.iter().cloned().collect(),
        }
    }
}

/// A Resource.
#[derive(TS, Serialize, Clone, Debug, Default)]
pub struct Resource {
    /// The ID of this Resource.
    pub id: Uuid,
    /// The name of this Resource.
    pub instance_name: String,
    /// The unique type name of this Resource.
    pub type_name: String,
    /// The id of the parent resource group.
    pub parent_group_id: Uuid,
}

impl Resource {
    /// Creates a UI resource with presentation data derived by the application analyzer.
    pub fn from_analyzed(
        value: &(impl a::resource::Resource + ?Sized),
        instance_name: impl Into<String>,
        parent_group_id: Uuid,
    ) -> Self {
        Self {
            id: value.id(),
            instance_name: instance_name.into(),
            type_name: value.type_name().to_owned(),
            parent_group_id,
        }
    }
}

/// A type of [`ResourceGroup`].
#[derive(TS, Serialize, Clone, Debug, Default)]
pub struct ResourceGroupTypeDecl {
    /// The name of the type of Resource Group
    pub name: String,
    /// The type names of the entities that used Resource of this group.
    pub used_by_entity_types: Vec<String>,
    /// The resource type names in this group or its descendants.
    pub contains_resource_types: Vec<String>,
}

/// A Group of [`Resource`]s.
#[derive(TS, Serialize, Clone, Debug, Default)]
pub struct ResourceGroup {
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

impl ResourceGroup {
    /// Creates a UI resource group from an analyzed Reference Tree entity.
    pub fn from_analyzed(
        value: &(impl Entity + ?Sized),
        instance_name: impl Into<String>,
        parent_group_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: value.id(),
            instance_name: instance_name.into(),
            type_name: value.type_name().to_owned(),
            parent_group_id,
        }
    }
}

/// A resource group node in a resource tree.
#[derive(TS, Serialize)]
pub struct ResourceGroupNode<T> {
    pub id: T,
    pub children: Vec<ResourceTree<T>>,
}

/// A tree of resources.
#[derive(TS, Serialize)]
pub enum ResourceTree<T> {
    ResourceGroup(ResourceGroupNode<T>),
    Resource(T),
}

pub fn convert_resource_tree<M>(
    node: ResourceTreeNode,
    model: &M,
) -> AnalyzerResult<Option<ResourceTree<<M as Model>::EntityIdType>>>
where
    M: Model,
    <M as Model>::EntityIdType: TS + Serialize,
{
    let ResourceTreeNode {
        entity_id,
        is_resource,
        children,
    } = node;
    let entity_ref = model.try_entity_ref(entity_id)?;
    if is_resource {
        if !children.is_empty() {
            return Err(a::AnalyzerError::Validation(
                "legacy UI resource tree cannot represent a resource with children".to_owned(),
            ));
        }
        return Ok(Some(ResourceTree::Resource(entity_ref)));
    }

    let children: Vec<ResourceTree<<M as Model>::EntityIdType>> = children
        .into_iter()
        .map(|child| convert_resource_tree(child, model))
        .collect::<AnalyzerResult<Vec<Option<ResourceTree<<M as Model>::EntityIdType>>>>>()?
        .into_iter()
        .flatten()
        .collect();
    if children.is_empty() {
        Ok(None)
    } else {
        Ok(Some(ResourceTree::ResourceGroup(ResourceGroupNode {
            id: entity_ref,
            children,
        })))
    }
}

/// A capacity value used by an FSM state.
#[derive(TS, Serialize, Clone, Debug)]
pub struct FsmUsage {
    /// The resource ID being used.
    pub resource: Uuid,
    /// The capacities being used (name, optional value).
    pub capacities: Vec<(String, Option<u64>)>,
}

impl From<&a::fsm::runtime::RtFsmStateUsage> for FsmUsage {
    fn from(value: &a::fsm::runtime::RtFsmStateUsage) -> Self {
        Self {
            resource: value.resource,
            capacities: value
                .capacities
                .iter()
                .map(|capacity| (capacity.name.to_string(), capacity.value))
                .collect(),
        }
    }
}

/// A transition in an FSM.
#[derive(TS, Serialize, Clone, Debug)]
pub struct FsmTransition {
    /// The name of the state this transition enters.
    pub name: String,
    /// The usages of this state.
    pub usages: Vec<FsmUsage>,
    /// The timestamp in seconds relative to an epoch.
    pub timestamp: TimeSec,
    /// Attributes recorded by the application's instrumentation.
    pub attributes: Vec<DynamicAttribute>,
    /// Attributes computed by the application's analyzer (e.g. a per-span
    /// rate), rendered separately from the recorded ones.
    pub derived_attributes: Vec<DynamicAttribute>,
}

impl FsmTransition {
    pub fn try_from_rt(
        value: &a::fsm::runtime::RtFsmTransition,
        epoch: TimeUnixNanoSec,
    ) -> Result<Self, quent_time::TimeError> {
        Ok(Self {
            name: value.name.clone(),
            usages: value.usages.iter().map(FsmUsage::from).collect(),
            timestamp: try_to_secs_relative(value.timestamp, epoch)?,
            attributes: value.attributes.clone(),
            derived_attributes: Vec::new(),
        })
    }
}

/// A run-time defined Finite-State-Machine.
#[derive(TS, Serialize, Clone, Debug)]
pub struct FiniteStateMachine {
    /// The ID of this FSM.
    pub id: Uuid,
    /// The type name of this FSM.
    pub type_name: String,
    /// The instance name of this FSM.
    pub instance_name: String,
    /// The transitions of this FSM.
    pub transitions: Vec<FsmTransition>,
}

impl FiniteStateMachine {
    pub fn try_from_rt(
        value: &a::fsm::runtime::RtFsm,
        epoch: TimeUnixNanoSec,
    ) -> Result<Self, quent_time::TimeError> {
        Ok(Self {
            id: value.id(),
            type_name: value.type_name().to_owned(),
            instance_name: value.instance_name().to_owned(),
            transitions: value
                .transitions()
                .iter()
                .map(|transition| FsmTransition::try_from_rt(transition, epoch))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

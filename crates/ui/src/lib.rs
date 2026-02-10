use quent_analyzer::{self as a, resource::CapacityType};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

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

impl<T: a::resource::Resource> From<&T> for Resource {
    fn from(value: &T) -> Self {
        Self {
            id: value.id(),
            instance_name: value.instance_name().to_owned(),
            type_name: value.type_name().to_owned(),
            parent_group_id: value.parent_group_id().to_owned(),
        }
    }
}

impl From<&dyn a::resource::Resource> for Resource {
    fn from(value: &dyn a::resource::Resource) -> Self {
        Self {
            id: value.id(),
            instance_name: value.instance_name().to_owned(),
            type_name: value.type_name().to_owned(),
            parent_group_id: value.parent_group_id().to_owned(),
        }
    }
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

impl From<&dyn a::resource::ResourceGroup> for ResourceGroup {
    fn from(value: &dyn a::resource::ResourceGroup) -> Self {
        Self {
            id: value.id(),
            instance_name: value.instance_name().to_owned(),
            type_name: value.type_name().to_owned(),
            parent_group_id: value.parent_group_id(),
        }
    }
}

/// A Group of [`Resource`]s.
#[derive(TS, Serialize, Clone, Debug, Default)]
pub struct ResourceTypeDecl {
    /// The unique type name for this type of Resource.
    pub name: String,
    /// The capacities of this type of Resource.
    pub capacities: Vec<String>,
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
                .map(|cap| match cap.kind {
                    CapacityType::Occupancy => cap.name.clone(),
                    CapacityType::Rate => format!("{}/s", cap.name),
                })
                .collect(),
            used_by: value.used_by.iter().cloned().collect(),
        }
    }
}

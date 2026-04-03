// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::resource::{channel::ChannelEvent, memory::MemoryEvent, processor::ProcessorEvent};

use super::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct Resource {
    pub instance_name: String,
    pub type_name: String, // TODO(johanpel): for now solve this like so, but this could be generated code too
    pub parent_group_id: Uuid,
}

pub mod memory {
    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Init {
        pub resource: Resource,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Operating {
        pub capacity_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Resizing {
        pub requested_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {
        pub unreclaimed_bytes: u64,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum MemoryEvent {
        Init(Init),
        Operating(Operating),
        Resizing(Resizing),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod processor {

    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Init {
        pub resource: Resource,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Operating {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum ProcessorEvent {
        Init(Init),
        Operating(Operating),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

pub mod channel {
    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Init {
        pub resource: Resource,
        pub source_id: Uuid,
        pub target_id: Uuid,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Operating {
        pub capacity_bytes: Option<u64>,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Finalizing {}

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct Exit {}

    #[derive(Debug, Deserialize, Serialize)]
    pub enum ChannelEvent {
        Init(Init),
        Operating(Operating),
        Finalizing(Finalizing),
        Exit(Exit),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupEvent {
    pub type_name: String,
    pub instance_name: String,
    pub parent_group_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ResourceEvent {
    Group(GroupEvent),
    Memory(MemoryEvent),
    Processor(ProcessorEvent),
    Channel(ChannelEvent),
}

pub mod r#use {
    use super::*;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Allocation {
        pub resource_id: Uuid,
        pub used_bytes: u64,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Transfer {
        pub resource_id: Uuid,
        pub used_bytes: u64,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Computation {
        pub resource_id: Uuid,
    }
}

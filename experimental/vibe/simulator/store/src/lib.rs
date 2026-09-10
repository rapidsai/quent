// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Schema-generated simulator stored-event types.

include!(concat!(env!("OUT_DIR"), "/simulator.rs"));

use quent_analyzer::{
    fsm::events::{AnalyzableTransition, AnalyzedUsage, DynamicAttribute},
    resource::CapacityValue,
};
use smallvec::{SmallVec, smallvec};

// TODO(johanpel): Generate these `AnalyzableTransition` implementations from
// schema metadata.
impl AnalyzableTransition for QueryEvent {
    fn entity_type_name() -> &'static str {
        "query"
    }

    fn sequence(&self) -> u16 {
        match self {
            Self::Init { seq, .. }
            | Self::Planning { seq }
            | Self::Executing { seq }
            | Self::Done { seq } => *seq,
        }
    }

    fn is_final(&self) -> bool {
        matches!(self, Self::Done { .. })
    }

    fn state_name(&self) -> &'static str {
        match self {
            Self::Init { .. } => "init",
            Self::Planning { .. } => "planning",
            Self::Executing { .. } => "executing",
            Self::Done { .. } => "done",
        }
    }

    fn instance_name(&self) -> Option<String> {
        match self {
            Self::Init { instance_name, .. } => Some(instance_name.clone()),
            _ => None,
        }
    }
}

impl AnalyzableTransition for TaskEvent {
    fn entity_type_name() -> &'static str {
        "task"
    }

    fn sequence(&self) -> u16 {
        match self {
            Self::Queueing { seq, .. }
            | Self::Allocating { seq, .. }
            | Self::Loading { seq, .. }
            | Self::Computing { seq, .. }
            | Self::Spilling { seq, .. }
            | Self::Sending { seq, .. }
            | Self::Exit { seq } => *seq,
        }
    }

    fn is_final(&self) -> bool {
        matches!(self, Self::Exit { .. })
    }

    fn state_name(&self) -> &'static str {
        match self {
            Self::Queueing { .. } => "queueing",
            Self::Allocating { .. } => "allocating",
            Self::Loading { .. } => "loading",
            Self::Computing { .. } => "computing",
            Self::Spilling { .. } => "spilling",
            Self::Sending { .. } => "sending",
            Self::Exit { .. } => "exit",
        }
    }

    fn instance_name(&self) -> Option<String> {
        match self {
            Self::Queueing { instance_name, .. } => Some(instance_name.clone()),
            _ => None,
        }
    }

    fn usages(&self) -> SmallVec<[AnalyzedUsage; 1]> {
        let unit = |resource_id| AnalyzedUsage {
            resource_id,
            capacities: SmallVec::new(),
        };
        let bytes = |resource_id, value| AnalyzedUsage {
            resource_id,
            capacities: smallvec![CapacityValue::new("bytes", value)],
        };
        match self {
            Self::Queueing { .. } | Self::Exit { .. } => SmallVec::new(),
            Self::Allocating { use_thread, .. } => smallvec![unit(use_thread.target)],
            Self::Loading {
                use_thread,
                use_storage_channel,
                use_pcie_channel,
                use_host_memory,
                use_gpu_memory,
                ..
            } => {
                let mut usages = smallvec![unit(use_thread.target)];
                usages.extend(
                    use_storage_channel
                        .iter()
                        .map(|usage| bytes(usage.target, usage.data.bytes)),
                );
                usages.extend(
                    use_pcie_channel
                        .iter()
                        .map(|usage| bytes(usage.target, usage.data.bytes)),
                );
                usages.extend(
                    use_host_memory
                        .iter()
                        .map(|usage| bytes(usage.target, usage.data.bytes)),
                );
                usages.extend(
                    use_gpu_memory
                        .iter()
                        .map(|usage| bytes(usage.target, usage.data.bytes)),
                );
                usages
            }
            Self::Computing {
                use_thread,
                use_host_memory,
                use_gpu_memory,
                ..
            } => {
                let mut usages = smallvec![unit(use_thread.target)];
                usages.extend(
                    use_host_memory
                        .iter()
                        .map(|usage| bytes(usage.target, usage.data.bytes)),
                );
                usages.extend(
                    use_gpu_memory
                        .iter()
                        .map(|usage| bytes(usage.target, usage.data.bytes)),
                );
                usages
            }
            Self::Spilling {
                use_thread,
                use_storage_channel,
                ..
            } => smallvec![
                unit(use_thread.target),
                bytes(use_storage_channel.target, use_storage_channel.data.bytes),
            ],
            Self::Sending {
                use_thread,
                use_network_channel,
                ..
            } => smallvec![
                unit(use_thread.target),
                bytes(use_network_channel.target, use_network_channel.data.bytes),
            ],
        }
    }

    fn dynamic_attributes(&self) -> Vec<DynamicAttribute> {
        match self {
            Self::Computing { input_bytes, .. } => {
                vec![DynamicAttribute::u64("input_bytes", *input_bytes)]
            }
            _ => Vec::new(),
        }
    }
}

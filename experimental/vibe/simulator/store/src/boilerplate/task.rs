// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use super::*;

impl TransitionEvent for TaskEvent {
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

    fn is_initial(&self) -> bool {
        matches!(self, Self::Queueing { .. })
    }

    fn is_final(&self) -> bool {
        matches!(self, Self::Exit { .. })
    }

    fn is_valid_next(&self, next: &Self) -> bool {
        match self {
            Self::Queueing { .. } => matches!(next, Self::Allocating { .. }),
            Self::Allocating { .. } => {
                matches!(next, Self::Computing { .. } | Self::Loading { .. })
            }
            Self::Loading { .. } => {
                matches!(next, Self::Computing { .. } | Self::Loading { .. })
            }
            Self::Computing { .. } => matches!(
                next,
                Self::Sending { .. } | Self::Spilling { .. } | Self::Exit { .. }
            ),
            Self::Spilling { .. } => matches!(next, Self::Allocating { .. }),
            Self::Sending { .. } => {
                matches!(next, Self::Queueing { .. } | Self::Exit { .. })
            }
            Self::Exit { .. } => false,
        }
    }

    fn name(&self) -> &'static str {
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

    fn usages(&self) -> SmallVec<[AnalyzedUsage; 1]> {
        let unit = |resource_id| AnalyzedUsage {
            resource_id,
            capacities: smallvec![CapacityValue::new("unit", 1)],
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
}

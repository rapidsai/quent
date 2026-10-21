// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_instrumentation_build_external_test::{annotation_message, predefined};
use quent_store::Timestamped;

/// Returns mark messages in the supplied event order.
pub fn annotation_messages(events: &[Timestamped<predefined::AnnotationsEvent>]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| annotation_message(&event.data))
        .collect()
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/// Binding a generated payload to a different entity name fails to compile.
///
/// ```compile_fail,E0080
/// mod incompatible {
///     include!(concat!(env!("OUT_DIR"), "/application_bad_name.rs"));
/// }
/// ```
#[allow(unused)]
pub mod predefined {
    include!(concat!(env!("OUT_DIR"), "/predefined.rs"));
}

/// Returns the message text for a mark event.
pub fn annotation_message(event: &predefined::AnnotationsEvent) -> Option<&str> {
    match event {
        predefined::AnnotationsEvent::Mark { message } => Some(&message.text),
        _ => None,
    }
}

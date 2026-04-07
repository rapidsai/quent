// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests for `define_context!` macro.

// Minimal event type for context
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TestEvent {
    Ping,
}

quent_model::define_context!(pub TestContext(TestEvent));

#[test]
fn define_context_struct_exists() {
    // Verify the generated struct has the expected methods.
    // We cannot actually create a context without an exporter, but we can
    // verify the type and its API exist.
    let _: fn(
        Option<quent_model::ExporterOptions>,
        uuid::Uuid,
    ) -> Result<TestContext, Box<dyn std::error::Error>> = TestContext::try_new;
}

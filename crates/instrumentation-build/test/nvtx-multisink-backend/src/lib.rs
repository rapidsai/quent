// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Binding-aware test backend for generated NVTX capture.

use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use nvtx_events::NvtxEvent;
use uuid::Uuid;

type Hook = Arc<dyn Fn(NvtxEvent) + Send + Sync + 'static>;

/// Identifies one logical generated capture source.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceBinding {
    pub context_id: Uuid,
    pub process_id: Uuid,
    pub stream_id: Uuid,
}

struct Registration {
    binding: SourceBinding,
    hook: Hook,
}

static REGISTRATIONS: Mutex<Vec<Registration>> = Mutex::new(Vec::new());

/// Accept every logical source so the fixture can exercise concurrent contexts.
pub fn register_source<F>(binding: SourceBinding, hook: F) -> Result<(), Infallible>
where
    F: Fn(NvtxEvent) + Send + Sync + 'static,
{
    REGISTRATIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(Registration {
            binding,
            hook: Arc::new(hook),
        });
    Ok(())
}

/// Return the bindings in registration order.
pub fn bindings() -> Vec<SourceBinding> {
    REGISTRATIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .map(|registration| registration.binding)
        .collect()
}

/// Deliver an event only to the source with this exact binding.
pub fn dispatch_to(binding: SourceBinding, event: NvtxEvent) -> bool {
    let hook = REGISTRATIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .find(|registration| registration.binding == binding)
        .map(|registration| Arc::clone(&registration.hook));
    let Some(hook) = hook else {
        return false;
    };
    hook(event);
    true
}

/// Clear fixture-global registrations between tests.
pub fn reset() {
    REGISTRATIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
}

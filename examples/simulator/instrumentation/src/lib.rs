// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_instrumentation::trace::TraceObserver;
use quent_simulator_events::SimulatorEvent;
use uuid::Uuid;

// Generate the context struct with try_new() and events_sender().
quent_model::define_context!(pub SimulatorContext(SimulatorEvent));

impl SimulatorContext {
    pub fn trace_observer(&self, entity_id: Uuid) -> TraceObserver<SimulatorEvent> {
        TraceObserver::new(self.events_sender(), entity_id)
    }
}

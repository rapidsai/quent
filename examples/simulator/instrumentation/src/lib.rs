// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_simulator_events::SimulatorEvent;

// Generate the context struct with try_new() and events_sender().
quent_model::define_context!(pub SimulatorContext(SimulatorEvent));

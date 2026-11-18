// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Browser-hosted API facade for the simulator analyzer.

use std::collections::HashSet;

use quent_events::Event;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use quent_query_engine_browser::{BrowserError, BrowserService};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_store::SimulatorEvent;

fn load_browser_service(bytes: &[u8]) -> Result<BrowserService<SimulatorUiAnalyzer>, BrowserError> {
    let bundle = quent_browser_bundle::decode(bytes).map_err(|error| {
        BrowserError::failure(format!("unable to decode browser bundle: {error}"))
    })?;
    if bundle.metadata.model_name != "quent-simulator" {
        return Err(BrowserError::failure(format!(
            "browser bundle model mismatch: expected quent-simulator, got {}",
            bundle.metadata.model_name
        )));
    }
    if bundle.metadata.codec != "postcard" {
        return Err(BrowserError::failure(format!(
            "unsupported simulator event codec: {}",
            bundle.metadata.codec
        )));
    }

    let events: Vec<Event<SimulatorEvent>> =
        postcard::from_bytes(bundle.payload).map_err(|error| {
            BrowserError::failure(format!("unable to decode simulator events: {error}"))
        })?;
    let engine_ids = events
        .iter()
        .filter_map(|event| matches!(event.data, SimulatorEvent::Engine(_)).then_some(event.id))
        .collect::<HashSet<_>>();
    if engine_ids.len() != 1 {
        return Err(BrowserError::failure(format!(
            "expected exactly one simulator engine, found {}",
            engine_ids.len()
        )));
    }
    let engine_id = *engine_ids.iter().next().expect("one engine was validated");
    if engine_id != bundle.metadata.engine_id {
        return Err(BrowserError::failure(
            "browser bundle engine ID does not match its events",
        ));
    }
    let analyzer =
        SimulatorUiAnalyzer::try_new(engine_id, events.into_iter()).map_err(BrowserError::from)?;
    Ok(BrowserService::new(
        analyzer,
        engine_id,
        bundle.metadata.context_ids,
    ))
}

quent_query_engine_browser::export_wasm_browser_server!(SimulatorUiAnalyzer, load_browser_service);

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use quent_dynamic_attributes::DynamicAttribute;
use quent_model::EventCallback;
use quent_query_engine_analyzer::ui::UiAnalyzer;
use quent_query_engine_fixed as fixed;
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_instrumentation::SimulatorContext;

#[test]
fn query_bundle_retains_arbitrary_observations_in_timestamp_order() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    {
        let captured = Arc::clone(&recorded);
        let context = SimulatorContext::try_new(EventCallback::new(move |event| {
            captured.lock().unwrap().push(event);
        }))
        .unwrap();
        fixed::emit(&context);
    }

    let events = std::mem::take(&mut *recorded.lock().unwrap());
    let analyzer = SimulatorUiAnalyzer::try_new(fixed::ENGINE, events.into_iter()).unwrap();
    let bundle = analyzer.query_bundle(fixed::QUERY).unwrap();
    let observations = &bundle.entities.operators[&fixed::PHYS_FINAL_AGG].observations;

    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.kind.as_str())
            .collect::<Vec<_>>(),
        ["algorithm.choice", "vendor.snapshot"]
    );
    assert_eq!(
        observations[0].custom_attributes,
        [DynamicAttribute::string("choice", "left")]
    );
    assert_eq!(
        observations[1].custom_attributes,
        [DynamicAttribute::u64("retained_rows", 42)]
    );
    assert_eq!(observations[0].port_relations.len(), 1);
    assert_eq!(
        observations[0].port_relations[0].port_id,
        fixed::PORT_PHYS_FINAL_AGG_IN
    );
    assert_eq!(observations[0].port_relations[0].role, "primary");
    assert!(observations[1].port_relations.is_empty());
}

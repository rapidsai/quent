// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use quent_nvtx_analyzer::NvtxModelBuilder;

mod fixtures;

#[test]
fn captured_sink_events_are_consumed_directly_by_analysis() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let (context, capture) = fixtures::start({
        let events = Arc::clone(&events);
        move |event| events.lock().unwrap().push(event)
    });
    nvtx::mark(c"captured mark");
    let range = nvtx::Range::new(c"captured range");
    drop(range);
    drop(context);
    drop(capture);

    let events = std::mem::take(&mut *events.lock().unwrap());
    let model = NvtxModelBuilder::build(events);
    assert_eq!(model.spans().len(), 1);
    assert_eq!(model.spans()[0].name, "captured range");
    assert!(model.spans()[0].end.is_some());
    assert_eq!(model.marks().len(), 1);
    assert_eq!(model.marks()[0].name, "captured mark");
    assert!(model.anomalies().is_faithful());
}

// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Handle resolution: registered strings, per-domain category namespacing,
//! placeholder stability, and the model surface those names hang off.

mod fixtures;

use fixtures::{at, range_end, range_start};
use nvtx_analyzer::{NvtxDomain, NvtxModel, NvtxModelBuilder};
use nvtx_bridge::NvtxEventEntity;
use nvtx_events::{NvtxEvent, NvtxEventAttributes, NvtxMessage};
use quent_events::Event;
use quent_time::TimeUnixNanoSec;

/// Attributes naming a previously (or subsequently) registered string.
fn registered(handle: u64) -> NvtxEventAttributes {
    NvtxEventAttributes {
        message: Some(NvtxMessage::RegisteredHandle(handle)),
        ..Default::default()
    }
}

/// A `RangeStart` whose message is a registered-string handle.
fn registered_start(
    timestamp: TimeUnixNanoSec,
    domain: u64,
    range_id: u64,
    handle: u64,
) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::RangeStart {
            domain,
            range_id,
            attributes: registered(handle),
        },
    )
}

/// A `RegisterString` binding `handle` to `string` within `domain`.
fn register(
    timestamp: TimeUnixNanoSec,
    domain: u64,
    handle: u64,
    string: &str,
) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::RegisterString {
            domain,
            handle,
            string: string.to_owned(),
        },
    )
}

/// A `NameCategory` naming `category` within `domain`.
fn name_category(
    timestamp: TimeUnixNanoSec,
    domain: u64,
    category: u32,
    name: &str,
) -> Event<NvtxEventEntity> {
    at(
        timestamp,
        NvtxEvent::NameCategory {
            domain,
            category,
            name: name.to_owned(),
        },
    )
}

/// The domain record for `domain`.
fn domain_record(model: &NvtxModel, domain: u64) -> &NvtxDomain {
    model
        .domains()
        .iter()
        .find(|record| record.domain == domain)
        .unwrap_or_else(|| panic!("no domain record for 0x{domain:X}"))
}

#[test]
fn uncreated_domain_reports_no_creation_time() {
    // Domain 7 is referenced but its `DomainCreate` was never captured — it
    // already existed when capture attached. `first_seen` is an upper bound on
    // when it was really created, and must not be reported as the creation.
    let events = vec![range_start(400, 7, 1, "work"), range_end(500, 7, 1)];

    let model = NvtxModelBuilder::build(events);

    let domain = domain_record(&model, 7);
    assert_eq!(
        domain.created, None,
        "no creation was observed, so none is claimed"
    );
    assert_eq!(domain.first_seen, 400);
    assert_eq!(domain.destroyed, None);
}

#[test]
fn resolve_registered_string() {
    // The registration arrives *after* the range that uses it: pass 1 scans the
    // whole stream, so a forward reference resolves like any other.
    let events = vec![
        registered_start(100, 1, 7, 0xAB),
        range_end(200, 1, 7),
        register(300, 1, 0xAB, "gather_kernel"),
        // The same handle value in a different domain is a *different* string.
        registered_start(400, 2, 8, 0xAB),
        range_end(500, 2, 8),
        register(600, 2, 0xAB, "scatter_kernel"),
    ];

    let model = NvtxModelBuilder::build(events);

    let names: Vec<&str> = model
        .spans()
        .iter()
        .map(|span| span.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["gather_kernel", "scatter_kernel"],
        "registered strings resolve per (domain, handle), never by bare handle"
    );
}

#[test]
fn category_namespaced_by_domain() {
    // Category 7 is named in two domains. A globally-keyed table would collapse
    // these into one name; the (domain, category) key keeps them distinct.
    let events = vec![
        name_category(10, 1, 7, "io"),
        name_category(20, 2, 7, "compute"),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.category_name(1, 7).as_deref(), Some("io"));
    assert_eq!(model.category_name(2, 7).as_deref(), Some("compute"));
    assert_eq!(
        model.category_name(0, 0),
        None,
        "category 0 is the 'no category' sentinel, not an unresolved reference"
    );

    let categories: Vec<(u64, u32, &str)> = model
        .categories()
        .iter()
        .map(|record| (record.domain, record.category, record.name.as_str()))
        .collect();
    assert_eq!(categories, vec![(1, 7, "io"), (2, 7, "compute")]);
}

#[test]
fn placeholder_stable() {
    // Nothing in this stream is ever registered, created, named, or destroyed:
    // every label below is a placeholder.
    let stream = || {
        vec![
            registered_start(100, 0x2A, 7, 0xBEEF),
            at(
                150,
                NvtxEvent::RangeEnd {
                    domain: 0x2A,
                    range_id: 7,
                },
            ),
            // Domain 0 is legitimately unnamed, and category 9 in domain 0x2A is
            // referenced but never named.
            at(
                200,
                NvtxEvent::Mark {
                    domain: 0,
                    attributes: NvtxEventAttributes {
                        category: 9,
                        ..Default::default()
                    },
                },
            ),
            // A thread that emits but is never named.
            at(
                250,
                NvtxEvent::RangePop {
                    domain: 0x2A,
                    thread_id: 42,
                },
            ),
        ]
    };

    let model = NvtxModelBuilder::build(stream());

    assert_eq!(
        model.spans()[0].name,
        "<unregistered string 0xBEEF>",
        "an unregistered handle surfaces its raw value"
    );
    assert_eq!(
        domain_record(&model, 0x2A).name,
        "<domain 0x2A>",
        "a referenced-but-uncreated domain surfaces its raw handle"
    );
    assert_eq!(
        domain_record(&model, 0).name,
        "default domain",
        "domain 0 is legitimately unnamed, not unresolved"
    );
    assert_eq!(
        model.category_name(0x2A, 9).as_deref(),
        Some("<category 9 @ domain 0x2A>"),
        "an unnamed category is qualified by its domain"
    );
    assert_eq!(
        model.thread_name(42),
        "thread 42",
        "an unnamed thread is legitimately unnamed, not unresolved"
    );

    // Placeholders are pure functions of the raw id — no counters, no
    // timestamps — so a rebuild is byte-identical.
    let again = NvtxModelBuilder::build(stream());
    assert_eq!(model.spans(), again.spans());
    assert_eq!(model.domains(), again.domains());
    assert_eq!(model.threads(), again.threads());
    assert_eq!(model.categories(), again.categories());
}

#[test]
fn model_surface_present() {
    let events = vec![
        at(
            10,
            NvtxEvent::DomainCreate {
                domain: 1,
                name: "cudf".to_owned(),
            },
        ),
        register(20, 1, 0x55, "checkpoint"),
        name_category(30, 1, 3, "io"),
        at(
            40,
            NvtxEvent::NameThread {
                thread_id: 42,
                name: "worker".to_owned(),
            },
        ),
        at(
            50,
            NvtxEvent::Mark {
                domain: 1,
                attributes: NvtxEventAttributes {
                    category: 3,
                    message: Some(NvtxMessage::RegisteredHandle(0x55)),
                    ..Default::default()
                },
            },
        ),
        at(90, NvtxEvent::DomainDestroy { domain: 1 }),
    ];

    let model = NvtxModelBuilder::build(events);

    assert_eq!(model.marks().len(), 1, "the Mark became an instant");
    let mark = &model.marks()[0];
    assert_eq!(mark.name, "checkpoint", "the mark's handle resolved");
    assert_eq!(mark.domain, 1);
    assert_eq!(mark.category, Some(3));
    assert_eq!(mark.timestamp, 50);
    assert!(
        model.spans().is_empty(),
        "a mark is an instant, never a zero-length span"
    );

    let domain = domain_record(&model, 1);
    assert_eq!(domain.name, "cudf");
    assert_eq!(
        domain.created,
        Some(10),
        "a captured DomainCreate is a real creation time"
    );
    assert_eq!(domain.first_seen, 10);
    assert_eq!(domain.destroyed, Some(90));

    assert_eq!(model.threads().len(), 1);
    assert_eq!(model.threads()[0].thread_id, 42);
    assert_eq!(model.threads()[0].name, "worker");
    assert_eq!(model.thread_name(42), "worker");

    assert_eq!(model.category_name(1, 3).as_deref(), Some("io"));
    assert_eq!(model.categories().len(), 1);
}

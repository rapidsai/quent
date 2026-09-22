// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;
use std::fmt::Display;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use nvtx_ui::{NvtxViewportRequest, NvtxViewportResponse};
use quent_analyzer::context::{ContextIndex, ContextInventory};
use quent_events::{EntityEvent, EntityRef, Event};
use quent_query_engine_analyzer::ui::{
    ContextMetadata, ContextStreamAvailability, ImportedContext,
};
use quent_query_engine_server::{error::ServerResult, model_viewer_router};
use quent_simulator_analyzer::Viewer;
use quent_simulator_store::{
    EngineEvent, EngineImplementationAttributes, NvtxEventEvent as NvtxEvent, RuntimeProcessEvent,
    SimulatorEvent,
    quent::{
        nvtx::{Attributes, Message},
        os,
    },
};
use tower::ServiceExt;
use uuid::Uuid;

const QUERY_START: u64 = 2_000_000_000;
const RANGE_END: u64 = 3_000_000_000;
const MAX_BODY_BYTES: usize = 64 * 1024;

fn catalog_uri(context_id: impl Display, query_start: u64) -> String {
    format!("/api/nvtx/contexts/{context_id}/catalog?query_start={query_start}")
}

fn viewport_uri(context_id: impl Display) -> String {
    format!("/api/nvtx/contexts/{context_id}/viewport?query_start={QUERY_START}")
}

fn attributes(message: &str) -> Attributes {
    Attributes {
        category: 0,
        color: None,
        payload: None,
        message: Some(Message {
            kind: 0,
            string: Some(message.to_owned()),
            registered_handle: None,
        }),
    }
}

type NvtxFixture = dyn Fn(Uuid) -> ServerResult<Option<Vec<Event<NvtxEvent>>>> + Send + Sync;

fn test_router(nvtx: Box<NvtxFixture>) -> Router {
    let importer = move |context_id: Uuid| {
        let captured = nvtx(context_id)?;
        let availability = match &captured {
            None => ContextStreamAvailability::Missing,
            Some(events) if events.is_empty() => ContextStreamAvailability::Empty,
            Some(_) => ContextStreamAvailability::Populated,
        };
        let process_id = Uuid::from_u128(context_id.as_u128() + 1000);
        let mut events = vec![
            Event::new(
                context_id,
                QUERY_START,
                SimulatorEvent::Engine(EngineEvent::Init {
                    implementation: EngineImplementationAttributes {
                        name: Some("test".into()),
                        version: None,
                        custom_attributes: Default::default(),
                    },
                    instance_name: None,
                }),
            ),
            Event::new(
                process_id,
                QUERY_START,
                SimulatorEvent::RuntimeProcess(RuntimeProcessEvent::Started {
                    process: os::Process { native_id: 42 },
                    engine_id: EntityRef::new(context_id, ()),
                }),
            ),
        ];
        if let Some(captured) = captured.filter(|events| !events.is_empty()) {
            events.push(Event::new(
                context_id,
                QUERY_START,
                SimulatorEvent::NvtxEvent(NvtxEvent::Initialized {
                    process: EntityRef::new(process_id, ()),
                }),
            ));
            events.extend(captured.into_iter().map(|event| {
                Event::new(
                    event.id,
                    event.timestamp,
                    SimulatorEvent::NvtxEvent(event.data),
                )
            }));
        }
        Ok(ImportedContext::new(
            Box::new(events.into_iter()),
            ContextMetadata::new([(NvtxEvent::NAME.to_owned(), availability)]),
        ))
    };
    let lister = || {
        let mut index = ContextIndex::default();
        for id in 1..=10 {
            let id = Uuid::from_u128(id);
            index.add_inventory(
                id.into(),
                ContextInventory {
                    analysis_target_ids: BTreeSet::from([id]),
                },
            );
        }
        Ok(index)
    };
    model_viewer_router::<Viewer>(Box::new(importer), Box::new(lister), None).unwrap()
}

fn range_events(context_id: Uuid) -> Vec<Event<NvtxEvent>> {
    let attributes = attributes("work");
    vec![
        Event::new(
            context_id,
            QUERY_START,
            NvtxEvent::RangeStart {
                domain: 4,
                range_id: 9,
                attributes,
            },
        ),
        Event::new(
            context_id,
            RANGE_END,
            NvtxEvent::RangeEnd {
                domain: 4,
                range_id: 9,
            },
        ),
    ]
}

fn grouped_range_events(context_id: Uuid) -> Vec<Event<NvtxEvent>> {
    let range = |domain, message: &str| {
        Event::new(
            context_id,
            QUERY_START,
            NvtxEvent::RangeStart {
                domain,
                range_id: 9,
                attributes: attributes(message),
            },
        )
    };
    let range_end = |domain| {
        Event::new(
            context_id,
            RANGE_END,
            NvtxEvent::RangeEnd {
                domain,
                range_id: 9,
            },
        )
    };
    vec![
        Event::new(
            context_id,
            QUERY_START,
            NvtxEvent::DomainCreate {
                domain: 5,
                name: "CCCL".to_owned(),
            },
        ),
        Event::new(
            context_id,
            QUERY_START,
            NvtxEvent::DomainCreate {
                domain: 172,
                name: "CCCL".to_owned(),
            },
        ),
        range(5, "work from 5"),
        range(172, "work from 172"),
        range_end(5),
        range_end(172),
    ]
}

#[tokio::test]
async fn model_and_query_relative_catalogs_are_cached_end_to_end() {
    let present = Uuid::from_u128(1);
    let loads = Arc::new(AtomicUsize::new(0));
    let importer_loads = Arc::clone(&loads);
    let app = test_router(Box::new(move |context_id| {
        importer_loads.fetch_add(1, Ordering::SeqCst);
        Ok((context_id == present).then(|| range_events(context_id)))
    }));

    let response = app
        .clone()
        .oneshot(
            Request::get(catalog_uri(present, QUERY_START))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let catalog: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(catalog["trace_start"], 0.0);
    assert_eq!(catalog["trace_end"], 1.0);
    assert!(catalog.get("query_start").is_none());

    let alternate_origin = QUERY_START + 500_000_000;
    let alternate = app
        .clone()
        .oneshot(
            Request::get(catalog_uri(present, alternate_origin))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(alternate.into_body(), usize::MAX).await.unwrap();
    let alternate: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(alternate["trace_start"], -0.5);
    assert_eq!(alternate["trace_end"], 0.5);

    let request = NvtxViewportRequest {
        viewport: nvtx_ui::NvtxViewportWindow {
            start: 0.0,
            end: 1.0,
        },
        selections: vec![nvtx_ui::NvtxDomainSelection {
            domain_id: 4,
            category_ids: vec![],
            include_uncategorized: true,
        }],
    };
    let response = app
        .oneshot(
            Request::post(viewport_uri(present))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let viewport: NvtxViewportResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(viewport.statistics[0].total_duration, 1.0);
    assert_eq!(
        loads.load(Ordering::SeqCst),
        1,
        "one reconstruction per context"
    );
}

#[tokio::test]
async fn grouped_domain_contract_survives_http_round_trip() {
    let context_id = Uuid::from_u128(10);
    let app =
        test_router(Box::new(move |requested_context_id| {
            Ok((requested_context_id == context_id)
                .then(|| grouped_range_events(requested_context_id)))
        }));

    let response = app
        .clone()
        .oneshot(
            Request::get(catalog_uri(context_id, QUERY_START))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let catalog_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(catalog_json["domains"].as_array().unwrap().len(), 1);
    assert_eq!(catalog_json["domains"][0]["domain_id"], "5");
    assert_eq!(
        catalog_json["domains"][0]["source_domain_ids"],
        serde_json::json!(["5", "172"])
    );

    let request = NvtxViewportRequest {
        viewport: nvtx_ui::NvtxViewportWindow {
            start: 0.0,
            end: 1.0,
        },
        selections: vec![nvtx_ui::NvtxDomainSelection {
            domain_id: 5,
            category_ids: vec![],
            include_uncategorized: true,
        }],
    };
    let response = app
        .oneshot(
            Request::post(viewport_uri(context_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let viewport: NvtxViewportResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(viewport.domains.len(), 1);
    assert_eq!(viewport.domains[0].domain_id, 5);
    assert_eq!(viewport.domains[0].source_domain_ids, vec![5, 172]);
    assert_eq!(
        viewport.domains[0]
            .lanes
            .iter()
            .flat_map(|lane| lane.ranges.iter())
            .map(|range| range.source_domain_id)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([5, 172])
    );
}

#[tokio::test]
async fn absent_and_empty_streams_are_distinct() {
    let empty = Uuid::from_u128(2);
    let absent = Uuid::from_u128(3);
    let app = test_router(Box::new(move |context_id| {
        Ok((context_id == empty).then(Vec::new))
    }));

    let empty_response = app
        .clone()
        .oneshot(
            Request::get(catalog_uri(empty, QUERY_START))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(empty_response.status(), StatusCode::OK);

    let absent_response = app
        .oneshot(
            Request::get(catalog_uri(absent, QUERY_START))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(absent_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_selector_is_a_bounded_bad_request() {
    let present = Uuid::from_u128(4);
    let app = test_router(Box::new(move |context_id| {
        Ok((context_id == present).then(|| range_events(context_id)))
    }));
    let request = NvtxViewportRequest {
        viewport: nvtx_ui::NvtxViewportWindow {
            start: 0.0,
            end: 1.0,
        },
        selections: vec![nvtx_ui::NvtxDomainSelection {
            domain_id: 4,
            category_ids: vec![],
            include_uncategorized: false,
        }],
    };
    let response = app
        .oneshot(
            Request::post(viewport_uri(present))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn malformed_paths_and_oversized_requests_are_rejected() {
    let present = Uuid::from_u128(5);
    let app = test_router(Box::new(move |context_id| {
        Ok((context_id == present).then(|| range_events(context_id)))
    }));

    let invalid_uuid = app
        .clone()
        .oneshot(
            Request::get(catalog_uri("not-a-uuid", QUERY_START))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_uuid.status(), StatusCode::BAD_REQUEST);

    let missing_origin = app
        .clone()
        .oneshot(
            Request::get(format!("/api/nvtx/contexts/{present}/catalog"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_origin.status(), StatusCode::BAD_REQUEST);

    let oversized = app
        .oneshot(
            Request::post(viewport_uri(present))
                .header("content-type", "application/json")
                .body(Body::from(vec![b' '; MAX_BODY_BYTES + 1]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn importer_failures_are_redacted_and_isolated_by_context() {
    let failing = Uuid::from_u128(6);
    let present = Uuid::from_u128(7);
    let app = test_router(Box::new(move |context_id| {
        if context_id == failing {
            Err(quent_io::ImporterError::other(std::io::Error::other(
                "/secret/capture/path could not be read",
            ))
            .into())
        } else {
            Ok((context_id == present).then(|| range_events(context_id)))
        }
    }));

    let failed = app
        .clone()
        .oneshot(
            Request::get(catalog_uri(failing, QUERY_START))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let failed_body = to_bytes(failed.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&failed_body[..], b"NVTX data could not be loaded");

    let healthy = app
        .oneshot(
            Request::get(catalog_uri(present, QUERY_START))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(healthy.status(), StatusCode::OK);
}

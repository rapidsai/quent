// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Tests model-wide storage using events exported by the instrumentation example.

use quent_store::{
    Error, Id,
    context::ContextSet,
    entity::{BorrowedEventSequenceStore, EntityHandle, EntityStore, native::NativeStore},
    event::filesystem,
};

use demo::{
    Connection, Demo, DemoStore, Query, connection::ConnectionEvents, server::ServerEvents,
};

#[allow(unused_imports, dead_code)]
mod demo {
    include!(concat!(env!("OUT_DIR"), "/demo.rs"));
}

#[test]
fn loads_example_events_into_the_model_store() {
    let output = tempfile::tempdir().unwrap();
    let context_id = quent_instrumentation_build_example::run_with_ndjson(output.path()).unwrap();
    let loader = filesystem::Loader::<Demo>::new(output.path(), ContextSet::one(context_id));
    let store = DemoStore::try_from_loader(&loader).unwrap();
    assert_eq!(
        [
            store.entities::<Connection>().unwrap().count(),
            store.entities::<demo::Server>().unwrap().count(),
            store.entities::<demo::ThreadPool>().unwrap().count(),
            store.entities::<demo::Thread>().unwrap().count(),
            store.entities::<Query>().unwrap().count(),
        ],
        [1; 5]
    );

    let id = store.entities::<Connection>().unwrap().next().unwrap();
    let connection = store.entity(id).unwrap().unwrap();
    assert_eq!(connection.opened().unwrap().peer().host, "localhost");
    assert_eq!(
        connection
            .transfer()
            .map(|event| *event.bytes())
            .collect::<Vec<_>>(),
        [1234, 5678]
    );
    let server_ref = connection.opened().unwrap().host();
    let server = store.entity(server_ref).unwrap().unwrap();
    assert_eq!(server.booted().process().native_id, std::process::id());
    assert!(std::ptr::eq(
        server,
        store.entity(server.id()).unwrap().unwrap()
    ));
    assert_eq!(store.event_sequence(id).unwrap().events().len(), 5);

    let missing = Id::<Connection>::new(demo::Uuid::nil());
    assert!(store.entity(missing).unwrap().is_none());
    assert!(
        matches!(store.event_sequence(missing), Err(Error::MissingEntity(id)) if id == missing.uuid())
    );

    fn analyze(store: &impl EntityStore<Connection, Error = Error>, id: Id<Connection>) {
        assert_eq!(store.entities().unwrap().count(), 1);
        assert_eq!(store.entity(id).unwrap().unwrap().num_events().get(), 5);
    }
    let entity_store = NativeStore::<Connection>::try_from_loader(&loader).unwrap();
    analyze(&entity_store, id);
    analyze(&store, id);
}

#[test]
fn propagates_loader_errors() {
    let output = tempfile::tempdir().unwrap();
    let context_id = demo::Uuid::nil();
    let loader = filesystem::Loader::<Demo>::new(output.path(), ContextSet::one(context_id));
    assert!(
        matches!(DemoStore::try_from_loader(&loader), Err(Error::ContextNotFound(id)) if id == context_id)
    );
}

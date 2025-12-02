//! Quent Instrumentation API
//!
use std::sync::RwLock;

use quent_events::{Event, EventData, Timestamp};
use quent_exporter::Exporter;
use quent_exporter_collector::CollectorExporter;
use tokio::runtime::{Handle, Runtime};
use uuid::Uuid;

#[inline]
fn timestamp() -> Timestamp {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;
    // Narrowing conversion to u64 limits this to Unix timestamp in seconds: 18446744073709551617
    // Which is in the 26th century
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_nanos() as u64)
        .unwrap_or_default()
    // TODO: consider to do something else instead of unwrap_or_default
}

struct Context {
    runtime: Handle,
    exporter: Box<dyn Exporter>,
}

// this is probably best moved to some ffi layer depending on the target lang
static CONTEXT: RwLock<Option<Context>> = RwLock::new(None);

impl Context {
    async fn try_new(runtime_handle: Handle) -> Result<Self, Box<dyn std::error::Error>> {
        let exporter = Box::new(CollectorExporter::new().await?);

        Ok(Context {
            runtime: runtime_handle,
            exporter,
        })
    }
}

// TODO(johanpel): minimize latency
fn push_event(event: Event<EventData>) {
    let read = CONTEXT.read().unwrap();
    if let Some(ctx) = read.as_ref() {
        let handle = &ctx.runtime;
        let exporter = &ctx.exporter;
        match handle.block_on(async move { exporter.push(event).await }) {
            Ok(_) => (),
            Err(e) => eprintln!("unable to send telemetry: {e}"),
        }
    }
}

// TODO: expose these through FFI:

/// Initialize the Quent Instrumentation API.
///
/// This must be called before anything else.
pub fn initialize() -> Result<(), Box<dyn std::error::Error>> {
    let handle = if let Ok(handle) = Handle::try_current() {
        eprintln!("using existing async runtime");
        handle
    } else {
        eprintln!("spawning new async runtime");
        if let Ok(runtime) = Runtime::new() {
            runtime.handle().clone()
        } else {
            eprintln!("unable to spawn async runtime");
            panic!("for now :tm:");
        }
    };

    let mut lock = CONTEXT.write()?;
    let context = handle.block_on(Context::try_new(handle.clone()))?;
    *lock = Some(context);
    Ok(())
}

// TODO(johanpel): boilerplate stuff below is to be filled in with more attribs
pub mod engine {
    use quent_events::engine;

    use super::*;

    pub fn init(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            engine::EngineEvent::Init(engine::Init {}).into(),
        ))
    }

    pub fn operating(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            engine::EngineEvent::Operating(engine::Operating {}).into(),
        ))
    }

    pub fn finalizing(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            engine::EngineEvent::Finalizing(engine::Finalizing {}).into(),
        ))
    }

    pub fn exit(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            engine::EngineEvent::Exit(engine::Exit {}).into(),
        ))
    }
}

pub mod coordinator {
    use quent_events::coordinator;

    use super::*;

    pub fn init(id: Uuid, engine_id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            coordinator::CoordinatorEvent::Init(coordinator::Init { engine_id }).into(),
        ))
    }

    pub fn operating(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            coordinator::CoordinatorEvent::Operating(coordinator::Operating {}).into(),
        ))
    }

    pub fn finalizing(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            coordinator::CoordinatorEvent::Finalizing(coordinator::Finalizing {}).into(),
        ))
    }

    pub fn exit(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            coordinator::CoordinatorEvent::Exit(coordinator::Exit {}).into(),
        ))
    }
}

pub mod query {
    use quent_events::query;

    use super::*;

    pub fn init(id: Uuid, coordinator_id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            query::QueryEvent::Init(query::Init { coordinator_id }).into(),
        ))
    }

    pub fn planning(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            query::QueryEvent::Planning(query::Planning {}).into(),
        ))
    }

    pub fn executing(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            query::QueryEvent::Executing(query::Executing {}).into(),
        ));
    }

    pub fn idle(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            query::QueryEvent::Idle(query::Idle {}).into(),
        ));
    }

    pub fn finalizing(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            query::QueryEvent::Finalizing(query::Finalizing {}).into(),
        ))
    }

    pub fn exit(id: Uuid) {
        push_event(Event::new(
            id,
            timestamp(),
            query::QueryEvent::Exit(query::Exit {}).into(),
        ))
    }
}

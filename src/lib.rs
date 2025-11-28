use std::{mem::ManuallyDrop, sync::LazyLock};

use quent_events::{Event, Timestamp, engine};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};
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
    _runtime: Runtime,
    thread: JoinHandle<()>,
    tx: ManuallyDrop<Sender<Event>>,
}

static CONTEXT: LazyLock<Option<Context>> = LazyLock::new(|| Context::new().ok());

impl Context {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dbg!("Initializing Context");

        let _runtime = tokio::runtime::Runtime::new()?;

        let (tx, mut rx): (Sender<Event>, Receiver<Event>) =
            tokio::sync::mpsc::channel(1024 * 1024);

        // TODO: context main thread spawning goes here

        let thread = _runtime.spawn_blocking(move || {
            loop {
                if let Some(event) = rx.blocking_recv() {
                    println!("{event:?}")
                    // TODO: batching and sending over gRPC logic goes here
                } else {
                    break;
                }
            }
        });

        Ok(Context {
            _runtime,
            thread,
            tx: ManuallyDrop::new(tx),
        })
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.tx) }
    }
}

// TODO: expose these through FFI:

pub fn engine_init(id: Uuid) {
    if let Some(ctx) = CONTEXT.as_ref() {
        if let Ok(_) = ctx
            .tx
            .try_send(Event::Engine(engine::Event::Init(engine::Init {
                id: id,
                t: timestamp(),
            })))
        {}
    }
}

// pub fn engine_operating(id: Uuid) {}
// pub fn engine_finalizing(id: Uuid) {}
// pub fn engine_exit(id: Uuid) {}

#[cfg(test)]
mod test {
    use crate::engine_init;

    #[test]
    pub fn client() {
        engine_init(uuid::Uuid::now_v7());
    }
}

use std::sync::{LazyLock, Mutex, RwLock};

use quent_collector::client::Client;
use quent_events::{Event, Timestamp, engine};
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
    collector_client: Client,
}

// this is probably best moved to some ffi layer depending on the target lang
static CONTEXT: RwLock<Option<Context>> = RwLock::new(None);

impl Context {
    async fn try_new() -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::new().await?;

        Ok(Context {
            collector_client: client,
        })
    }
}

// TODO: expose these through FFI:

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
    let context = handle.block_on(Context::try_new())?;
    *lock = Some(context);
    Ok(())
}

pub fn engine_init(id: Uuid) {
    let read = CONTEXT.read().unwrap();
    if let Some(ctx) = read.as_ref() {
        let handle = tokio::runtime::Handle::current();
        let client = &ctx.collector_client;
        if let Ok(_) = handle.block_on(async move {
            client
                .send(Event::Engine(engine::Event::Init(engine::Init {
                    id,
                    t: timestamp(),
                })))
                .await
        }) {}
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

use tracing::info;

fn initialize_tracing() {
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_target(true)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::new("quent=debug,simulator=debug")) // only show 'quent' targets
        .init();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    let engine_id = uuid::Uuid::now_v7();

    info!("simulating an engine with id: {engine_id}");

    let context = quent::Context::try_new(engine_id)?;

    info!("context created, creating events...");

    let engine_obs = context.engine_observer();

    engine_obs.init(engine_id);
    engine_obs.operating(engine_id);

    let coordinator_futures: Vec<_> = std::iter::repeat_with(|| uuid::Uuid::now_v7())
        .take(2)
        .map(|coordinator| {
            std::thread::spawn({
                let engine_id = engine_id.clone();
                let coordinator_obs = context.coordinator_observer();
                let query_obs = context.query_observer();
                move || {
                    coordinator_obs.init(coordinator, engine_id);
                    coordinator_obs.operating(coordinator);

                    let query_futures: Vec<_> = std::iter::repeat_with(|| uuid::Uuid::now_v7())
                        .take(3)
                        .map(|query| {
                            std::thread::spawn({
                                let query_obs = query_obs.clone();
                                move || {
                                    query_obs.init(query, coordinator);
                                    query_obs.planning(query);
                                    query_obs.executing(query);
                                    query_obs.idle(query);
                                    query_obs.finalizing(query);
                                    query_obs.exit(query);
                                }
                            })
                        })
                        .collect();

                    for query_future in query_futures {
                        query_future.join().unwrap();
                    }

                    coordinator_obs.finalizing(coordinator);
                    coordinator_obs.exit(coordinator);
                }
            })
        })
        .collect();

    for coordinator_future in coordinator_futures {
        coordinator_future.join().unwrap();
    }

    engine_obs.finalizing(engine_id);
    engine_obs.exit(engine_id);

    info!("events pushed, waiting 1s to flush (for now :tm:)");
    // TODO(johanpel): ensure the channels are flushed on drop
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}

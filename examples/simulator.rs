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

    let engine = uuid::Uuid::now_v7();

    info!("simulating an engine with id: {engine}");

    let _ = quent::initialize(engine);

    quent::engine::init(engine);
    quent::engine::operating(engine);

    let coordinator_futures: Vec<_> = std::iter::repeat_with(|| uuid::Uuid::now_v7())
        .take(2)
        .map(|coordinator| {
            std::thread::spawn({
                let engine = engine.clone();
                move || {
                    quent::coordinator::init(coordinator, engine);
                    quent::coordinator::operating(coordinator);

                    let query_futures: Vec<_> = std::iter::repeat_with(|| uuid::Uuid::now_v7())
                        .take(3)
                        .map(|query| {
                            std::thread::spawn({
                                let coordinator = coordinator.clone();
                                move || {
                                    quent::query::init(query, coordinator);
                                    quent::query::planning(query);
                                    quent::query::executing(query);
                                    quent::query::idle(query);
                                    quent::query::finalizing(query);
                                    quent::query::exit(query);
                                }
                            })
                        })
                        .collect();

                    for query_future in query_futures {
                        query_future.join().unwrap();
                    }

                    quent::coordinator::finalizing(coordinator);
                    quent::coordinator::exit(coordinator);
                }
            })
        })
        .collect();

    for coordinator_future in coordinator_futures {
        coordinator_future.join().unwrap();
    }

    quent::engine::finalizing(engine);
    quent::engine::exit(engine);

    info!("events pushed, waiting 1s to flush (for now :tm:)");
    // TODO(johanpel): ensure the channels are flushed on drop
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}

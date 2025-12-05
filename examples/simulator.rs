use quent_events::{
    coordinator,
    engine::{self, EngineImplementationAttributes},
    query, worker,
};
use rand::{Rng, rng};
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

    // Spawn engine
    let engine_obs = context.engine_observer();
    engine_obs.init(
        engine_id,
        engine::Init {
            name: Some(format!("holodeck-{:04x}", rng().random::<u32>())),
            implementation: Some(EngineImplementationAttributes {
                name: Some("Simulator".into()),
                version: Some("0.0.0-PoC".into()),
            }),
        },
    );

    // Spawn workers
    let worker_obs = context.worker_observer();
    let worker_ids = std::iter::repeat_with(|| uuid::Uuid::now_v7())
        .take(4)
        .collect::<Vec<_>>();
    for (worker_index, worker_id) in worker_ids.iter().enumerate() {
        worker_obs.init(
            *worker_id,
            worker::Init {
                engine_id,
                name: Some(format!("worker-{worker_index}")),
            },
        );
    }
    for worker_id in worker_ids.iter() {
        worker_obs.operating(*worker_id, worker::Operating {});
    }

    engine_obs.operating(engine_id, engine::Operating {});

    let coordinator_futures: Vec<_> = std::iter::repeat_with(|| uuid::Uuid::now_v7())
        .take(2)
        .map(|coordinator_id| {
            std::thread::spawn({
                let engine_id = engine_id.clone();
                let coordinator_obs = context.coordinator_observer();
                let query_obs = context.query_observer();
                move || {
                    coordinator_obs.init(
                        coordinator_id,
                        coordinator::Init {
                            engine_id,
                            name: Some(format!("coordinator-{:04x}", rng().random::<u32>())),
                        },
                    );
                    coordinator_obs.operating(coordinator_id, coordinator::Operating {});

                    let query_futures: Vec<_> = std::iter::repeat_with(|| uuid::Uuid::now_v7())
                        .take(3)
                        .map(|query_id| {
                            std::thread::spawn({
                                let query_obs = query_obs.clone();
                                move || {
                                    query_obs.init(
                                        query_id,
                                        query::Init {
                                            coordinator_id,
                                            name: Some(format!("query-{}", rng().random::<u32>())),
                                        },
                                    );
                                    query_obs.planning(query_id, query::Planning {});
                                    query_obs.executing(query_id, query::Executing {});
                                    query_obs.idle(query_id, query::Idle {});
                                    query_obs.finalizing(query_id, query::Finalizing {});
                                    query_obs.exit(query_id, query::Exit {});
                                }
                            })
                        })
                        .collect();

                    for query_future in query_futures {
                        query_future.join().unwrap();
                    }

                    coordinator_obs.finalizing(coordinator_id, coordinator::Finalizing {});
                    coordinator_obs.exit(coordinator_id, coordinator::Exit {});
                }
            })
        })
        .collect();

    for coordinator_future in coordinator_futures {
        coordinator_future.join().unwrap();
    }

    engine_obs.finalizing(engine_id, engine::Finalizing {});

    // Shut down workers.
    for worker_id in worker_ids.iter() {
        worker_obs.finalizing(*worker_id, worker::Finalizing {});
        worker_obs.exit(*worker_id, worker::Exit {});
    }

    engine_obs.exit(engine_id, engine::Exit {});

    info!("events pushed, waiting 1s to flush (for now :tm:)");
    // TODO(johanpel): ensure the channels are flushed on drop
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}

use std::net::ToSocketAddrs;

use quent_collector::{proto::collector_server::CollectorServer, server::CollectorService};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".to_socket_addrs().unwrap().next().unwrap();
    let collector = CollectorService::default();

    let client = tokio::task::spawn_blocking(|| {
        // TODO(johanpel): give some time to spawn the server, but this isn't pretty,
        // we should give the client a means to retry a bunch of times
        std::thread::sleep(std::time::Duration::from_secs(1));

        let engine = uuid::Uuid::now_v7();

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
    });

    let _server = Server::builder()
        .add_service(CollectorServer::new(collector))
        .serve(addr)
        .await?;
    eprintln!("spawned collector service on {addr}");

    client.await?;

    Ok(())
}

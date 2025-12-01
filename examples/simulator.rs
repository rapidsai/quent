use std::net::ToSocketAddrs;

use quent_collector::{
    client::Client, proto::collector_server::CollectorServer, server::CollectorService,
};
use quent_events::Event;
use tokio::task::JoinHandle;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".to_socket_addrs().unwrap().next().unwrap();
    let collector = CollectorService::default();

    let client = tokio::task::spawn_blocking(|| {
        // TODO(johanpel): give some time to spawn the server, but this isn't pretty,
        // we should give the client a means to retry a bunch of times
        std::thread::sleep(std::time::Duration::from_secs(1));

        let _ = quent::initialize();
        quent::engine_init(uuid::Uuid::now_v7());
    });

    let _server = Server::builder()
        .add_service(CollectorServer::new(collector))
        .serve(addr)
        .await?;
    eprintln!("spawned collector service on {addr}");

    client.await?;

    Ok(())
}

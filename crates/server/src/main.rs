use std::net::ToSocketAddrs;

use quent_collector::{proto::collector_server::CollectorServer, server::CollectorService};
use tonic::transport::Server;
use tracing::info;

fn initialize_tracing() {
    use tracing_subscriber::{
        layer::SubscriberExt,
        util::SubscriberInitExt,
        {EnvFilter, fmt},
    };

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")))
        .with(fmt::layer().with_target(true).with_writer(std::io::stderr))
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    initialize_tracing();

    // TODO(johanpel): configuruation
    let addr = "[::1]:50051".to_socket_addrs().unwrap().next().unwrap();
    let collector = CollectorService::default();

    info!("spawning collector service on {addr}, send SIGINT (e.g. ctrl+c) to exit");
    let _server = Server::builder()
        .add_service(CollectorServer::new(collector))
        .serve(addr)
        .await?;

    Ok(())
}

use std::net::ToSocketAddrs;

use quent_client::Client;
use quent_collector::CollectorService;
use quent_events::Event;
use quent_proto::collector_server::CollectorServer;
use tokio::task::JoinHandle;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".to_socket_addrs().unwrap().next().unwrap();
    let collector = CollectorService::default();

    let client = tokio::spawn(async {
        // TODO(johanpel): give some time to spawn the server, but this isn't pretty,
        // we should give the client a means to retry a bunch of times
        std::thread::sleep(std::time::Duration::from_secs(1));

        let mut client = Client::new().await?;
        eprintln!("spawned client");

        let send = client.send(Event::Flush).await?;

        Ok::<(), quent_client::Error>(())
    });

    let server = Server::builder()
        .add_service(CollectorServer::new(collector))
        .serve(addr)
        .await?;
    eprintln!("spawned collector service on {addr}");

    client.await?;

    Ok(())
}

//! Client and server code for the Collector service.
//!
//! This allows multiple sources to send events to a centralized place, where it can be further processed / exported.
#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod server;

pub mod proto {
    tonic::include_proto!("quent.collector.v1");
}

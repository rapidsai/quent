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

pub mod env {
    pub const QUENT_COLLECTOR_ADDRESS: &str = "QUENT_COLLECTOR_ADDRESS";
}

pub mod default {
    pub const QUENT_COLLECTOR_PORT: u16 = 7836;
}

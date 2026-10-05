// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Python lifecycle binding for a model-specific collector server.

use std::future::Future;
use std::net::{Ipv6Addr, SocketAddr, TcpListener};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use pyo3::{
    exceptions::{PyOSError, PyRuntimeError, PyTimeoutError, PyValueError},
    prelude::*,
    types::PyAny,
};
use quent_collector::{
    CollectorSink,
    server::{CollectorService, FlushHandle},
};
use quent_collector_proto::collector_server::CollectorServer;
use tokio::{runtime::Runtime, sync::oneshot, task::JoinHandle};
use tokio_stream::{Stream, wrappers::TcpListenerStream};
use tonic::transport::Server;
use uuid::Uuid;

/// Closes the listener before signaling Tonic to drain existing connections.
///
/// Tonic retains its incoming stream during draining, so the listener must close
/// explicitly to prevent clients reconnecting to a socket that is no longer polled.
struct ShutdownIncoming {
    listener: Option<TcpListenerStream>,
    shutdown: oneshot::Receiver<()>,
    draining: Option<oneshot::Sender<()>>,
}

impl Stream for ShutdownIncoming {
    type Item = std::io::Result<tokio::net::TcpStream>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.listener.is_none() {
            return Poll::Ready(None);
        }
        if Pin::new(&mut self.shutdown).poll(cx).is_ready() {
            self.listener.take();
            if let Some(draining) = self.draining.take() {
                let _ = draining.send(());
            }
            return Poll::Ready(None);
        }
        Pin::new(self.listener.as_mut().unwrap()).poll_next(cx)
    }
}

/// Owns the runtime used to serve requests and complete shutdown.
struct ServerHandle {
    shutdown: oneshot::Sender<()>,
    runtime: Runtime,
    task: JoinHandle<Result<(), String>>,
    flush: FlushHandle,
}

impl ServerHandle {
    /// Stops the server and waits for exporter shutdown before dropping its runtime.
    fn close(self) -> Result<(), String> {
        let Self {
            shutdown,
            runtime,
            task,
            flush,
        } = self;
        let _ = shutdown.send(());
        let result = runtime.block_on(task).map_err(|error| error.to_string());
        flush.wait();
        result?
    }

    fn close_with_timeout(self, timeout: Duration) -> PyResult<()> {
        let started = Instant::now();
        let Self {
            shutdown,
            runtime,
            task,
            flush,
        } = self;
        let _ = shutdown.send(());
        let result = runtime.block_on(async {
            tokio::time::timeout(timeout.saturating_sub(started.elapsed()), task).await
        });
        let flushed =
            result.is_ok() && flush.wait_timeout(timeout.saturating_sub(started.elapsed()));
        runtime.shutdown_timeout(timeout.saturating_sub(started.elapsed()));
        if !flushed || started.elapsed() >= timeout {
            return Err(PyTimeoutError::new_err(
                "collector shutdown timed out; pending events may be lost",
            ));
        }
        result
            .unwrap()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?
            .map_err(PyRuntimeError::new_err)
    }
}

/// Owns a collector server running on a dedicated Rust runtime.
///
/// Call `close()` or use `with` to wait for shutdown. Discarding the object
/// starts shutdown in the background without waiting for it to finish.
#[pyclass(name = "Collector")]
pub struct Collector {
    address: String,
    handle: Option<ServerHandle>,
}

/// Binds a nonblocking listener and returns its validated advertised HTTP address.
fn bind_listener(
    bind_address: &str,
    advertised_host: Option<&str>,
) -> PyResult<(TcpListener, String)> {
    let bind_address: SocketAddr = bind_address.parse().map_err(|error| {
        PyValueError::new_err(format!("invalid collector bind address: {error}"))
    })?;
    let host = match advertised_host {
        Some(host) if !host.is_empty() => host.to_owned(),
        Some(_) => return Err(PyValueError::new_err("advertised_host cannot be empty")),
        None if bind_address.ip().is_unspecified() => {
            return Err(PyValueError::new_err(
                "advertised_host is required for a wildcard bind address",
            ));
        }
        None => bind_address.ip().to_string(),
    };
    let listener = TcpListener::bind(bind_address).map_err(PyOSError::new_err)?;
    let port = listener.local_addr().map_err(PyOSError::new_err)?.port();
    let uri_host = if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]")
    } else {
        host
    };
    let address = format!("http://{uri_host}:{port}");
    let uri: http::Uri = address
        .parse()
        .map_err(|_| PyValueError::new_err("invalid advertised_host"))?;
    if uri.host().is_none() || uri.port_u16() != Some(port) {
        return Err(PyValueError::new_err("invalid advertised_host"));
    }
    listener.set_nonblocking(true).map_err(PyOSError::new_err)?;
    Ok((listener, address))
}

impl Collector {
    /// Starts a server that builds one `C` per source context ID.
    ///
    /// The returned address uses `advertised_host`, or the bound IP if omitted.
    /// A wildcard bind requires an explicit advertised host.
    ///
    /// # Errors
    ///
    /// Returns a Python exception if an address is invalid or the server cannot start.
    pub fn start<C>(
        bind_address: &str,
        advertised_host: Option<&str>,
        make: impl Fn(Uuid) -> Result<C, String> + Send + Sync + 'static,
    ) -> PyResult<Self>
    where
        C: CollectorSink + Send + Sync + 'static,
    {
        let (listener, address) = bind_listener(bind_address, advertised_host)?;

        // The listener and gRPC request deadlines require I/O and time drivers.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(PyRuntimeError::new_err)?;
        let listener = {
            let _entered = runtime.enter();
            tokio::net::TcpListener::from_std(listener).map_err(PyOSError::new_err)?
        };
        let collector = CollectorService::new(make);
        let flush = collector.flush_handle();
        let (shutdown, shutdown_rx) = oneshot::channel();
        let (draining, draining_rx) = oneshot::channel();
        let incoming = ShutdownIncoming {
            listener: Some(TcpListenerStream::new(listener)),
            shutdown: shutdown_rx,
            draining: Some(draining),
        };
        let task = runtime.spawn(async move {
            Server::builder()
                .add_service(CollectorServer::new(collector))
                .serve_with_incoming_shutdown(incoming, async move {
                    let _ = draining_rx.await;
                })
                .await
                .map_err(|error| error.to_string())
        });
        Ok(Self {
            address,
            handle: Some(ServerHandle {
                shutdown,
                runtime,
                task,
                flush,
            }),
        })
    }
}

#[pymethods]
impl Collector {
    #[getter]
    /// Returns the HTTP URI used by collector clients.
    fn address(&self) -> &str {
        &self.address
    }

    #[getter]
    /// Reports whether shutdown has been requested.
    fn closed(&self) -> bool {
        self.handle.is_none()
    }

    /// Stops accepting streams and waits for local exporter shutdown attempts
    /// to finish.
    ///
    /// Close client contexts and release all their observers and handles first so
    /// sender-side buffers can flush and client streams can end.
    ///
    /// With `timeout=None`, this waits indefinitely for connections and local
    /// exporters to finish. This should only be used if it can be guaranteed
    /// that all connections will close and that all local exporters will
    /// finish.
    ///
    /// With `timeout=` this function will wait until either:
    /// 1. all connections are closed and all local exporter shutdown attempts
    ///    have finished, or
    /// 2. some finite, nonnegative number of seconds defined by `timeout`.
    ///
    /// If the timeout expires, remaining connections are forcefully closed and
    /// `TimeoutError` is raised. Unfinished exporter cleanup may continue in the
    /// background. Pending event data may be lost, including data buffered in
    /// local exporters.
    ///
    /// Repeated calls have no effect, including after a timeout.
    ///
    /// Invalid timeouts raise a `ValueError`.
    /// A server failure raises `RuntimeError`.
    #[pyo3(signature = (timeout=None))]
    fn close(&mut self, py: Python<'_>, timeout: Option<f64>) -> PyResult<()> {
        let timeout = timeout
            .map(Duration::try_from_secs_f64)
            .transpose()
            .map_err(|_| {
                PyValueError::new_err("timeout must be finite, nonnegative, and representable")
            })?;
        match self.handle.take() {
            Some(handle) => py.detach(|| match timeout {
                Some(timeout) => handle.close_with_timeout(timeout),
                None => handle.close().map_err(PyRuntimeError::new_err),
            }),
            None => Ok(()),
        }
    }

    /// Returns this collector for use in a Python `with` statement.
    fn __enter__(slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf
    }

    /// Closes the collector when a Python `with` statement exits.
    fn __exit__(
        &mut self,
        py: Python<'_>,
        _exc_type: &Bound<'_, PyAny>,
        _exc_value: &Bound<'_, PyAny>,
        _traceback: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        self.close(py, None)
    }
}

/// Requests shutdown without waiting when the collector is discarded.
impl Drop for Collector {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            std::thread::spawn(move || {
                let _ = handle.close();
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use quent_collector_proto::{CollectEventRequest, collector_client::CollectorClient};
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::ReceiverStream;
    use tonic::Request;

    struct Sink(mpsc::UnboundedSender<()>);

    impl CollectorSink for Sink {
        fn ingest(&self, _: &str, _: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
            self.0.send(())?;
            Ok(())
        }
    }

    #[test]
    fn shutdown_rejects_late_rpc_while_stream_is_open() {
        let (active_tx, mut active_rx) = mpsc::unbounded_channel();
        let mut collector =
            Collector::start("127.0.0.1:0", None, move |_| Ok(Sink(active_tx.clone()))).unwrap();
        let ServerHandle {
            shutdown,
            runtime: _server_runtime,
            task,
            flush,
        } = collector.handle.take().unwrap();
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), async {
                let mut late_client = CollectorClient::connect(collector.address.clone())
                    .await
                    .unwrap();
                let mut client = CollectorClient::connect(collector.address.clone())
                    .await
                    .unwrap();
                let (events_tx, events_rx) = mpsc::channel(1);
                let mut request = Request::new(ReceiverStream::new(events_rx));
                request.metadata_mut().insert(
                    "source-context-id",
                    Uuid::now_v7().to_string().parse().unwrap(),
                );
                request
                    .metadata_mut()
                    .insert("entity-type", "test".parse().unwrap());
                let rpc = tokio::spawn(async move { client.collect_events(request).await });
                events_tx
                    .send(CollectEventRequest {
                        event: vec![vec![]],
                    })
                    .await
                    .unwrap();
                active_rx.recv().await.unwrap();

                shutdown.send(()).unwrap();
                let address = collector.address.strip_prefix("http://").unwrap();
                while tokio::net::TcpStream::connect(address).await.is_ok() {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                late_client
                    .collect_events(Request::new(tokio_stream::empty::<CollectEventRequest>()))
                    .await
                    .unwrap_err();

                drop(events_tx);
                rpc.await.unwrap().unwrap();
                task.await.unwrap().unwrap();
            })
            .await
            .expect("collector shutdown hung");
        });
        flush.wait();
    }
}

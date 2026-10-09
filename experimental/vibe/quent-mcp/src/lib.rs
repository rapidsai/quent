// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Experimental MCP bridge for the Quent REST API.
//!
//! This crate deliberately mirrors REST operations without adding analysis
//! semantics. Deterministic reductions belong in `quent-cli`; diagnosis belongs
//! in the calling agent.

use std::{
    fmt::Display,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
};

use axum::Router;
use reqwest::{Client, RequestBuilder};
use rmcp::{
    ErrorData, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::{
        io::stdio,
        streamable_http_server::{
            session::local::LocalSessionManager,
            tower::{StreamableHttpServerConfig, StreamableHttpService},
        },
    },
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

const SERVER_NAME: &str = "quent-mcp";
const ENV_ALLOWED_HOSTS: &str = "QUENT_MCP_ALLOWED_HOSTS";
const INSTRUCTIONS: &str = "\
Quent exposes query-engine telemetry as raw, typed facts. Typical exploration flow:
1. `list_engines` to discover telemetry captures.
2. `list_query_groups` and `list_queries` to find a query.
3. `get_query` to inspect the query bundle and discover resource/operator ids.
4. Timeline, data-flow, or entity tools to retrieve scoped evidence.
These tools mirror the Quent REST API. Use quent-cli for deterministic summaries,
rankings, joins, fingerprints, or comparisons.";

#[derive(Clone)]
struct QuentRestClient {
    base_url: String,
    client: Client,
}

impl QuentRestClient {
    fn new(api_base: &str) -> Result<Self, String> {
        let base_url = api_base.trim().trim_end_matches('/');
        if base_url.is_empty() {
            return Err("Quent API base URL must not be empty".to_owned());
        }
        reqwest::Url::parse(base_url)
            .map_err(|error| format!("invalid Quent API base URL: {error}"))?;
        Ok(Self {
            base_url: base_url.to_owned(),
            client: Client::new(),
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    async fn get(&self, path: &str) -> Result<CallToolResult, ErrorData> {
        api_result(self.client.get(self.endpoint(path))).await
    }

    async fn get_with_metadata(
        &self,
        path: &str,
        with_metadata: bool,
    ) -> Result<CallToolResult, ErrorData> {
        api_result(
            self.client
                .get(self.endpoint(path))
                .query(&[("with_metadata", with_metadata)]),
        )
        .await
    }

    async fn post(&self, path: &str, request: &Value) -> Result<CallToolResult, ErrorData> {
        api_result(self.client.post(self.endpoint(path)).json(request)).await
    }
}

fn internal_error(error: impl Display) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}

async fn api_result(request: RequestBuilder) -> Result<CallToolResult, ErrorData> {
    let response = request.send().await.map_err(internal_error)?;
    let status = response.status();
    let body = response.text().await.map_err(internal_error)?;
    if !status.is_success() {
        return Err(internal_error(format!(
            "Quent API returned {status}: {}",
            body.trim()
        )));
    }
    let value: Value = serde_json::from_str(&body)
        .map_err(|error| internal_error(format!("Quent API returned invalid JSON: {error}")))?;
    Ok(CallToolResult::success(vec![Content::json(value)?]))
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, ErrorData> {
    Uuid::parse_str(value)
        .map_err(|error| ErrorData::invalid_params(format!("invalid {field}: {error}"), None))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListEnginesArgs {
    /// Include engine metadata such as names and durations.
    #[serde(default)]
    with_metadata: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct EngineArgs {
    /// Engine UUID.
    engine_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct QueryGroupArgs {
    /// Engine UUID.
    engine_id: String,
    /// Query-group UUID.
    query_group_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct QueryArgs {
    /// Engine UUID.
    engine_id: String,
    /// Query UUID.
    query_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct EngineRequestArgs {
    /// Engine UUID.
    engine_id: String,
    /// Request body matching the corresponding Quent REST endpoint.
    request: Value,
}

/// MCP server that faithfully proxies the Quent REST API.
#[derive(Clone)]
pub struct QuentMcpServer {
    api: QuentRestClient,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl QuentMcpServer {
    /// Create an MCP server for a Quent REST API base such as
    /// `http://localhost:8080/api`.
    pub fn new(api_base: &str) -> Result<Self, String> {
        Ok(Self {
            api: QuentRestClient::new(api_base)?,
            tool_router: Self::tool_router(),
        })
    }

    #[tool(description = "List Quent engines (telemetry capture sessions).")]
    async fn list_engines(
        &self,
        Parameters(args): Parameters<ListEnginesArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.api
            .get_with_metadata("engines", args.with_metadata)
            .await
    }

    #[tool(description = "Get one Quent engine and its metadata.")]
    async fn get_engine(
        &self,
        Parameters(args): Parameters<EngineArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        self.api.get(&format!("engines/{engine_id}")).await
    }

    #[tool(description = "List telemetry contexts attributed to one engine.")]
    async fn list_engine_contexts(
        &self,
        Parameters(args): Parameters<EngineArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        self.api.get(&format!("engines/{engine_id}/contexts")).await
    }

    #[tool(description = "List query groups belonging to one engine.")]
    async fn list_query_groups(
        &self,
        Parameters(args): Parameters<EngineArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        self.api
            .get(&format!("engines/{engine_id}/query-groups"))
            .await
    }

    #[tool(description = "List queries belonging to one query group.")]
    async fn list_queries(
        &self,
        Parameters(args): Parameters<QueryGroupArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        let query_group_id = parse_uuid(&args.query_group_id, "query_group_id")?;
        self.api
            .get(&format!(
                "engines/{engine_id}/query_group/{query_group_id}/queries"
            ))
            .await
    }

    #[tool(description = "Get one query bundle, including plans, operators, and resources.")]
    async fn get_query(
        &self,
        Parameters(args): Parameters<QueryArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        let query_id = parse_uuid(&args.query_id, "query_id")?;
        self.api
            .get(&format!("engines/{engine_id}/query/{query_id}"))
            .await
    }

    #[tool(description = "Fetch one binned resource or resource-group timeline.")]
    async fn single_timeline(
        &self,
        Parameters(args): Parameters<EngineRequestArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        self.api
            .post(
                &format!("engines/{engine_id}/timeline/single"),
                &args.request,
            )
            .await
    }

    #[tool(description = "Fetch multiple binned resource or resource-group timelines.")]
    async fn bulk_timelines(
        &self,
        Parameters(args): Parameters<EngineRequestArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        self.api
            .post(&format!("engines/{engine_id}/timeline/bulk"), &args.request)
            .await
    }

    #[tool(description = "Fetch a categorical per-operator data-flow timeline.")]
    async fn data_flow_timeline(
        &self,
        Parameters(args): Parameters<EngineRequestArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        self.api
            .post(
                &format!("engines/{engine_id}/timeline/data-flow"),
                &args.request,
            )
            .await
    }

    #[tool(description = "List entities matching resource, window, and application filters.")]
    async fn list_entities(
        &self,
        Parameters(args): Parameters<EngineRequestArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&args.engine_id, "engine_id")?;
        self.api
            .post(&format!("engines/{engine_id}/entities"), &args.request)
            .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for QuentMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut implementation = Implementation::from_build_env();
        implementation.name = SERVER_NAME.to_owned();
        implementation.version = env!("CARGO_PKG_VERSION").to_owned();

        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = implementation;
        info.instructions = Some(INSTRUCTIONS.to_owned());
        info
    }
}

#[derive(Debug, PartialEq, Eq)]
enum AllowedHosts {
    Default,
    Disabled,
    Explicit(Vec<String>),
}

fn allowed_hosts(value: Option<&str>) -> AllowedHosts {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return AllowedHosts::Default;
    };
    if value == "*" {
        return AllowedHosts::Disabled;
    }
    AllowedHosts::Explicit(
        value
            .split(',')
            .map(str::trim)
            .filter(|host| !host.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

fn http_config() -> StreamableHttpServerConfig {
    let mut config = StreamableHttpServerConfig::default();
    match allowed_hosts(std::env::var(ENV_ALLOWED_HOSTS).ok().as_deref()) {
        AllowedHosts::Default => {}
        AllowedHosts::Disabled => config = config.disable_allowed_hosts(),
        AllowedHosts::Explicit(hosts) => config = config.with_allowed_hosts(hosts),
    }
    config
}

/// Build a streamable-HTTP MCP service for mounting at `/mcp`.
pub fn http_service(
    api_base: &str,
) -> Result<StreamableHttpService<QuentMcpServer, LocalSessionManager>, String> {
    let server = QuentMcpServer::new(api_base)?;
    Ok(StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        http_config(),
    ))
}

/// Build the MCP routes for merging into an existing Axum server.
pub fn http_routes(api_base: &str) -> Result<Router, String> {
    Ok(Router::new().nest_service("/mcp", http_service(api_base)?))
}

/// Return the REST base reachable from a server bound at `address`.
pub fn local_api_base(address: SocketAddr) -> String {
    let ip = match address.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        ip => ip,
    };
    format!("http://{}/api", SocketAddr::new(ip, address.port()))
}

/// Serve MCP over stdin/stdout until the client disconnects.
pub async fn serve_stdio(api_base: &str) -> Result<(), Box<dyn std::error::Error>> {
    let service = QuentMcpServer::new(api_base)?.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        sync::{Arc, Mutex},
    };

    use axum::{
        Json, Router,
        extract::{Request, State},
        routing::any,
    };
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::*;

    const ENGINE_ID: &str = "018f06d0-7b3a-7cc1-9f5f-0800200c9a66";
    const QUERY_GROUP_ID: &str = "018f06d0-7b3a-7cc1-9f5f-0800200c9a67";
    const QUERY_ID: &str = "018f06d0-7b3a-7cc1-9f5f-0800200c9a68";

    #[derive(Clone, Default)]
    struct TestClient;

    impl rmcp::ClientHandler for TestClient {}

    async fn record(
        State(requests): State<Arc<Mutex<Vec<String>>>>,
        request: Request,
    ) -> Json<Value> {
        let target = request
            .uri()
            .path_and_query()
            .map(ToString::to_string)
            .unwrap_or_default();
        requests
            .lock()
            .expect("request log mutex should not be poisoned")
            .push(format!("{} {target}", request.method()));
        Json(json!({ "ok": true }))
    }

    async fn mock_api() -> (String, Arc<Mutex<Vec<String>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .fallback(any(record))
            .with_state(requests.clone());
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock API should bind");
        let address = listener
            .local_addr()
            .expect("mock API should have an address");
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("mock API should serve requests");
        });
        (format!("http://{address}/api"), requests)
    }

    #[test]
    fn normalizes_and_validates_api_base() {
        let server = QuentMcpServer::new("http://localhost:8080/api///")
            .expect("valid API base should be accepted");
        assert_eq!(server.api.base_url, "http://localhost:8080/api");
        assert!(QuentMcpServer::new("").is_err());
        assert!(QuentMcpServer::new("not a URL").is_err());
    }

    #[test]
    fn derives_reachable_api_base() {
        assert_eq!(
            local_api_base("0.0.0.0:8080".parse().unwrap()),
            "http://127.0.0.1:8080/api"
        );
        assert_eq!(
            local_api_base("[::]:8080".parse().unwrap()),
            "http://[::1]:8080/api"
        );
        assert_eq!(
            local_api_base("192.0.2.10:8080".parse().unwrap()),
            "http://192.0.2.10:8080/api"
        );
    }

    #[test]
    fn parses_allowed_host_policy() {
        assert_eq!(allowed_hosts(None), AllowedHosts::Default);
        assert_eq!(allowed_hosts(Some("")), AllowedHosts::Default);
        assert_eq!(allowed_hosts(Some("*")), AllowedHosts::Disabled);
        assert_eq!(
            allowed_hosts(Some("quent.test, 127.0.0.1")),
            AllowedHosts::Explicit(vec!["quent.test".to_owned(), "127.0.0.1".to_owned()])
        );
    }

    #[tokio::test]
    async fn rejects_invalid_ids_before_calling_api() {
        let (api_base, requests) = mock_api().await;
        let server = QuentMcpServer::new(&api_base).expect("server should initialize");
        let result = server
            .get_engine(Parameters(EngineArgs {
                engine_id: "not-a-uuid".to_owned(),
            }))
            .await;
        assert!(result.is_err());
        assert!(
            requests
                .lock()
                .expect("request log mutex should not be poisoned")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn mirrors_every_generic_rest_operation() {
        let (api_base, requests) = mock_api().await;
        let server = QuentMcpServer::new(&api_base).expect("server should initialize");

        server
            .list_engines(Parameters(ListEnginesArgs {
                with_metadata: true,
            }))
            .await
            .expect("list_engines should proxy");
        server
            .get_engine(Parameters(EngineArgs {
                engine_id: ENGINE_ID.to_owned(),
            }))
            .await
            .expect("get_engine should proxy");
        server
            .list_engine_contexts(Parameters(EngineArgs {
                engine_id: ENGINE_ID.to_owned(),
            }))
            .await
            .expect("list_engine_contexts should proxy");
        server
            .list_query_groups(Parameters(EngineArgs {
                engine_id: ENGINE_ID.to_owned(),
            }))
            .await
            .expect("list_query_groups should proxy");
        server
            .list_queries(Parameters(QueryGroupArgs {
                engine_id: ENGINE_ID.to_owned(),
                query_group_id: QUERY_GROUP_ID.to_owned(),
            }))
            .await
            .expect("list_queries should proxy");
        server
            .get_query(Parameters(QueryArgs {
                engine_id: ENGINE_ID.to_owned(),
                query_id: QUERY_ID.to_owned(),
            }))
            .await
            .expect("get_query should proxy");
        let request = || {
            Parameters(EngineRequestArgs {
                engine_id: ENGINE_ID.to_owned(),
                request: json!({ "fixture": true }),
            })
        };
        server
            .single_timeline(request())
            .await
            .expect("single_timeline should proxy");
        server
            .bulk_timelines(request())
            .await
            .expect("bulk_timelines should proxy");
        server
            .data_flow_timeline(request())
            .await
            .expect("data_flow_timeline should proxy");
        server
            .list_entities(request())
            .await
            .expect("list_entities should proxy");

        assert_eq!(
            *requests
                .lock()
                .expect("request log mutex should not be poisoned"),
            vec![
                "GET /api/engines?with_metadata=true".to_owned(),
                format!("GET /api/engines/{ENGINE_ID}"),
                format!("GET /api/engines/{ENGINE_ID}/contexts"),
                format!("GET /api/engines/{ENGINE_ID}/query-groups"),
                format!("GET /api/engines/{ENGINE_ID}/query_group/{QUERY_GROUP_ID}/queries"),
                format!("GET /api/engines/{ENGINE_ID}/query/{QUERY_ID}"),
                format!("POST /api/engines/{ENGINE_ID}/timeline/single"),
                format!("POST /api/engines/{ENGINE_ID}/timeline/bulk"),
                format!("POST /api/engines/{ENGINE_ID}/timeline/data-flow"),
                format!("POST /api/engines/{ENGINE_ID}/entities"),
            ]
        );
    }

    #[tokio::test]
    async fn stdio_compatible_transport_lists_only_rest_parity_tools() {
        let server =
            QuentMcpServer::new("http://localhost:8080/api").expect("server should initialize");
        let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
        let server_task = tokio::spawn(async move {
            let service = server
                .serve(server_transport)
                .await
                .expect("server transport should initialize");
            service
                .waiting()
                .await
                .expect("server transport should close cleanly");
        });
        let client = TestClient
            .serve(client_transport)
            .await
            .expect("client should connect");
        let tools = client
            .peer()
            .list_tools(None)
            .await
            .expect("tools/list should succeed");
        let names = tools
            .tools
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            names,
            [
                "bulk_timelines",
                "data_flow_timeline",
                "get_engine",
                "get_query",
                "list_engine_contexts",
                "list_engines",
                "list_entities",
                "list_queries",
                "list_query_groups",
                "single_timeline",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        client.cancel().await.expect("client should close cleanly");
        server_task.await.expect("server task should finish");
    }

    #[tokio::test]
    async fn streamable_http_transport_initializes() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("HTTP MCP listener should bind");
        let address = listener
            .local_addr()
            .expect("HTTP MCP listener should have an address");
        let app = http_routes("http://localhost:8080/api").expect("HTTP service should initialize");
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("HTTP MCP server should run");
        });

        let client = reqwest::Client::new();
        let initialize = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"quent-mcp-test","version":"1.0"}}}"#;
        let rejected = client
            .post(format!("http://{address}/mcp"))
            .header("Host", "untrusted.example")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(initialize)
            .send()
            .await
            .expect("disallowed Host request should complete");
        assert!(!rejected.status().is_success());

        let response = client
            .post(format!("http://{address}/mcp"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(initialize)
            .send()
            .await
            .expect("initialize request should complete");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(response.headers().contains_key("mcp-session-id"));
        task.abort();
    }
}

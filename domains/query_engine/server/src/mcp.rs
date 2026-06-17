// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Model Context Protocol (MCP) server exposing the analyzer surface as tools.
//!
//! This lets AI agents connect to a running quent analysis server and explore
//! telemetry data. The handler is generic over [`UiAnalyzer`], reusing the same
//! [`ServiceState`] (analyzer + timeline caches) as the REST API, so out-of-repo
//! analyzers get MCP support for free.
//!
//! Two transports are provided:
//! - [`http_service`] returns a tower service that mounts into the Axum analyzer
//!   router (e.g. at `/mcp`).
//! - [`serve_stdio`] runs the server over stdin/stdout for clients that spawn it
//!   as a subprocess.
//!
//! The tools mirror the REST endpoints (`list_engines`, `get_engine`,
//! `list_query_groups`, `list_queries`, `get_query`, `single_timeline`) and add
//! a few agent-friendly summaries (`engine_overview`, `plan_tree`,
//! `slowest_operators`).

use std::sync::Arc;

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
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use quent_analyzer::Entity;
use quent_query_engine_analyzer::{
    QueryEngineModel, plan::tree::PlanTree, query_group::QueryGroup, ui::UiAnalyzer,
};
use quent_query_engine_ui as ui;
use quent_ui::timeline::request::SingleTimelineRequest;

use crate::{
    analyzer_cache::{AnalyzerCache, ImporterFn, ListerFn},
    error::ServerError,
    state::ServiceState,
    timeline_cache::TimelineCache,
};

/// Default name reported to MCP clients.
const SERVER_NAME: &str = "quent-analyzer";

/// Env var to relax the HTTP transport's Host allowlist for non-local
/// deployments: a comma-separated list of allowed hosts, or `*` to disable the
/// check. Unset keeps rmcp's secure loopback-only default. See [`http_service`].
const ENV_ALLOWED_HOSTS: &str = "QUENT_MCP_ALLOWED_HOSTS";

/// Human-readable guidance shown to agents on connect.
const INSTRUCTIONS: &str = "\
Quent exposes query-engine telemetry for performance analysis. Typical exploration flow:
1. `list_engines` to discover capture sessions (engines). Pass with_metadata=true for names/durations.
2. `engine_overview` for a quick summary (counts + duration) of one engine.
3. `list_query_groups` then `list_queries` to drill into the queries an engine ran.
4. `get_query` for the full query bundle (entities, plan tree, resource tree).
5. `plan_tree` / `slowest_operators` for digested views of a single query.
6. `single_timeline` for binned resource-usage timelines (advanced; request shape matches the
   REST POST /api/engines/{engine_id}/timeline/single body — use get_query to find resource ids).
All ids are UUID strings.";

#[derive(Deserialize, JsonSchema)]
struct ListEnginesArg {
    /// Include engine metadata (name, duration). Slower: reads each engine's events.
    #[serde(default)]
    with_metadata: bool,
}

#[derive(Deserialize, JsonSchema)]
struct EngineArg {
    /// UUID of the engine.
    engine_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct QueryGroupArg {
    /// UUID of the engine.
    engine_id: String,
    /// UUID of the query group.
    query_group_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct QueryArg {
    /// UUID of the engine.
    engine_id: String,
    /// UUID of the query.
    query_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct SlowestOperatorsArg {
    /// UUID of the engine.
    engine_id: String,
    /// UUID of the query.
    query_id: String,
    /// Maximum number of operators to return (default 10).
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct SingleTimelineArg {
    /// UUID of the engine.
    engine_id: String,
    /// Timeline request body, matching the REST `POST /timeline/single` shape.
    request: serde_json::Value,
}

#[derive(Serialize)]
struct OperatorTiming {
    operator_id: Uuid,
    operator_type_name: Option<String>,
    instance_name: Option<String>,
    active_duration_s: Option<f64>,
}

/// Parse a UUID tool argument, mapping failures to an MCP invalid-params error.
fn parse_uuid(value: &str, field: &str) -> Result<Uuid, ErrorData> {
    Uuid::parse_str(value)
        .map_err(|e| ErrorData::invalid_params(format!("invalid {field}: {e}"), None))
}

/// Map any analyzer/server error into an MCP internal error.
fn err<E: Into<ServerError>>(e: E) -> ErrorData {
    ErrorData::internal_error(e.into().to_string(), None)
}

/// Serialize a value as JSON tool content.
fn json_ok<T: Serialize>(value: T) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::json(value)?]))
}

/// MCP server exposing the query-engine analyzer surface as tools.
pub struct QuentMcpServer<A: UiAnalyzer + Send + Sync + 'static> {
    state: ServiceState<A>,
    tool_router: ToolRouter<Self>,
}

impl<A> Clone for QuentMcpServer<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            tool_router: self.tool_router.clone(),
        }
    }
}

#[tool_router]
impl<A> QuentMcpServer<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::EntityRef: Serialize,
{
    /// Build a server over an existing analyzer service state.
    pub fn new(state: ServiceState<A>) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List all engines (telemetry capture sessions). Set with_metadata=true to \
                       include each engine's name and duration."
    )]
    async fn list_engines(
        &self,
        Parameters(arg): Parameters<ListEnginesArg>,
    ) -> Result<CallToolResult, ErrorData> {
        if arg.with_metadata {
            json_ok(
                self.state
                    .analyzers
                    .list_with_metadata()
                    .await
                    .map_err(err)?,
            )
        } else {
            let engines: Vec<ui::Engine> = self
                .state
                .analyzers
                .list()
                .map_err(err)?
                .into_iter()
                .map(ui::Engine::new)
                .collect();
            json_ok(engines)
        }
    }

    #[tool(description = "Get details (name, duration, implementation) for one engine by id.")]
    async fn get_engine(
        &self,
        Parameters(arg): Parameters<EngineArg>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&arg.engine_id, "engine_id")?;
        let analyzer = self.state.analyzers.get(engine_id).await.map_err(err)?;
        let engine = analyzer.query_engine_model().engine().map_err(err)?;
        json_ok(engine.to_ui().map_err(err)?)
    }

    #[tool(description = "List all query groups for an engine.")]
    async fn list_query_groups(
        &self,
        Parameters(arg): Parameters<EngineArg>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&arg.engine_id, "engine_id")?;
        let analyzer = self.state.analyzers.get(engine_id).await.map_err(err)?;
        let groups: Vec<ui::QueryGroup> = analyzer
            .query_engine_model()
            .query_groups()
            .map(QueryGroup::to_ui)
            .collect();
        json_ok(groups)
    }

    #[tool(description = "List all queries in a query group.")]
    async fn list_queries(
        &self,
        Parameters(arg): Parameters<QueryGroupArg>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&arg.engine_id, "engine_id")?;
        let query_group_id = parse_uuid(&arg.query_group_id, "query_group_id")?;
        let analyzer = self.state.analyzers.get(engine_id).await.map_err(err)?;
        let queries = analyzer
            .query_engine_model()
            .queries()
            .filter(|q| q.query_group_id() == Some(query_group_id))
            .map(|q| q.to_ui())
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        json_ok(queries)
    }

    #[tool(
        description = "Get the full query bundle for one query: entities, plan tree, and resource \
                       tree. Large; prefer plan_tree / slowest_operators for a quick look."
    )]
    async fn get_query(
        &self,
        Parameters(arg): Parameters<QueryArg>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&arg.engine_id, "engine_id")?;
        let query_id = parse_uuid(&arg.query_id, "query_id")?;
        let analyzer = self.state.analyzers.get(engine_id).await.map_err(err)?;
        json_ok(analyzer.query_bundle(query_id).map_err(err)?)
    }

    #[tool(
        description = "Summarize an engine: counts of query groups, queries, and workers, plus \
                       engine name and duration."
    )]
    async fn engine_overview(
        &self,
        Parameters(arg): Parameters<EngineArg>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&arg.engine_id, "engine_id")?;
        let analyzer = self.state.analyzers.get(engine_id).await.map_err(err)?;
        let model = analyzer.query_engine_model();
        let engine = model.engine().map_err(err)?.to_ui().map_err(err)?;
        json_ok(serde_json::json!({
            "engine": engine,
            "num_query_groups": model.query_groups().count(),
            "num_queries": model.queries().count(),
            "num_workers": model.workers().count(),
        }))
    }

    #[tool(
        description = "Render the plan tree of one query as indented text, annotating each plan \
                       with its worker and operator type names."
    )]
    async fn plan_tree(
        &self,
        Parameters(arg): Parameters<QueryArg>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&arg.engine_id, "engine_id")?;
        let query_id = parse_uuid(&arg.query_id, "query_id")?;
        let analyzer = self.state.analyzers.get(engine_id).await.map_err(err)?;
        let model = analyzer.query_engine_model();
        let tree = model.plan_tree(query_id).map_err(err)?;

        let mut out = String::new();
        render_plan_tree(model, &tree, 0, &mut out);
        Ok(CallToolResult::success(vec![Content::text(out)]))
    }

    #[tool(
        description = "List a query's operators ranked by active duration (longest first). \
                       Useful for spotting bottlenecks."
    )]
    async fn slowest_operators(
        &self,
        Parameters(arg): Parameters<SlowestOperatorsArg>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&arg.engine_id, "engine_id")?;
        let query_id = parse_uuid(&arg.query_id, "query_id")?;
        let limit = arg.limit.unwrap_or(10) as usize;
        let analyzer = self.state.analyzers.get(engine_id).await.map_err(err)?;
        let model = analyzer.query_engine_model();
        let epoch = model.query_epoch(query_id).map_err(err)?;
        let plans = model.query_plans(query_id).map_err(err)?;

        let mut timings: Vec<OperatorTiming> = model
            .plans_operators(plans)
            .map_err(err)?
            .map(|op| {
                let ui = op.to_ui(epoch);
                OperatorTiming {
                    operator_id: ui.id,
                    operator_type_name: ui.operator_type_name,
                    instance_name: ui.instance_name,
                    active_duration_s: ui.active_span.map(|span| span.duration()),
                }
            })
            .collect();

        // Longest active duration first; operators without a span sort last.
        timings.sort_by(|a, b| {
            let a = a.active_duration_s.unwrap_or(f64::NEG_INFINITY);
            let b = b.active_duration_s.unwrap_or(f64::NEG_INFINITY);
            b.total_cmp(&a)
        });
        timings.truncate(limit);
        json_ok(timings)
    }

    #[tool(
        description = "Fetch a single binned resource (or resource-group) timeline. The `request` \
                       field matches the REST POST /api/engines/{engine_id}/timeline/single body."
    )]
    async fn single_timeline(
        &self,
        Parameters(arg): Parameters<SingleTimelineArg>,
    ) -> Result<CallToolResult, ErrorData> {
        let engine_id = parse_uuid(&arg.engine_id, "engine_id")?;
        let request: SingleTimelineRequest<ui::QueryFilter, ui::OperatorFilter> =
            serde_json::from_value(arg.request).map_err(|e| {
                ErrorData::invalid_params(format!("invalid timeline request: {e}"), None)
            })?;
        let analyzer = self.state.analyzers.get(engine_id).await.map_err(err)?;
        let response = self
            .state
            .timelines
            .cached_single_timeline(analyzer, engine_id, request)
            .await
            .map_err(err)?;
        json_ok(response)
    }
}

#[tool_handler]
impl<A> ServerHandler for QuentMcpServer<A>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::EntityRef: Serialize,
{
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo`/`Implementation` are `#[non_exhaustive]`, so build via
        // defaults and field assignment rather than struct literals.
        let mut server_info = Implementation::from_build_env();
        server_info.name = SERVER_NAME.to_string();
        server_info.version = env!("CARGO_PKG_VERSION").to_string();

        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info.instructions = Some(INSTRUCTIONS.to_string());
        info
    }
}

/// Recursively render a [`PlanTree`] node and its children into `out`.
fn render_plan_tree<M: QueryEngineModel>(
    model: &M,
    node: &PlanTree,
    depth: usize,
    out: &mut String,
) {
    let indent = "  ".repeat(depth);
    let worker = node
        .worker
        .map(|w| format!(" (worker {w})"))
        .unwrap_or_default();
    out.push_str(&format!("{indent}plan {}{worker}\n", node.id));

    let mut operator_names: Vec<String> = model
        .operators()
        .filter(|op| op.plan_id() == Some(node.id))
        .map(|op| {
            op.operator_type_name()
                .map(str::to_string)
                .unwrap_or_else(|| format!("operator {}", op.id()))
        })
        .collect();
    operator_names.sort();
    if !operator_names.is_empty() {
        out.push_str(&format!(
            "{indent}  operators: {}\n",
            operator_names.join(", ")
        ));
    }

    for child in &node.children {
        render_plan_tree(model, child, depth + 1, out);
    }
}

/// Build a [`StreamableHttpService`] that can be mounted into an Axum router.
///
/// Mount it with `router.nest_service("/mcp", mcp::http_service(state))`.
pub fn http_service<A>(
    state: ServiceState<A>,
) -> StreamableHttpService<QuentMcpServer<A>, LocalSessionManager>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::EntityRef: Serialize,
{
    // rmcp's default Host allowlist is loopback-only — a DNS-rebinding guard
    // that stops a malicious web page from driving a victim's local analyzer.
    // Keep that secure default. Non-local deployments (reached via a hostname or
    // IP) opt in via `QUENT_MCP_ALLOWED_HOSTS`: a comma-separated host list, or
    // `*` to disable the check entirely (e.g. when fronted by an authenticating
    // proxy). See [`ENV_ALLOWED_HOSTS`].
    let mut config = StreamableHttpServerConfig::default();
    if let Ok(hosts) = std::env::var(ENV_ALLOWED_HOSTS) {
        let hosts = hosts.trim();
        config = if hosts == "*" {
            config.disable_allowed_hosts()
        } else {
            config.with_allowed_hosts(hosts.split(',').map(|h| h.trim().to_string()))
        };
    }
    StreamableHttpService::new(
        move || Ok(QuentMcpServer::new(state.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    )
}

/// Serve the MCP server over stdio until the client disconnects.
///
/// Builds the analyzer service state from `importer`/`lister` (the same
/// callbacks the HTTP server uses) and runs the protocol on stdin/stdout.
/// Logs must go to stderr — stdout carries the MCP protocol.
pub async fn serve_stdio<A>(
    importer: Box<ImporterFn<A>>,
    lister: Box<ListerFn>,
) -> Result<(), Box<dyn std::error::Error>>
where
    A: UiAnalyzer + Send + Sync + 'static,
    <A as UiAnalyzer>::EntityRef: Serialize,
{
    let state = ServiceState {
        analyzers: AnalyzerCache::<A>::new(importer, lister),
        timelines: TimelineCache::new(),
    };
    let service = QuentMcpServer::new(state).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

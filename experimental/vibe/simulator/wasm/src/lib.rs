// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Browser-hosted API facade for the simulator analyzer.

use quent_analyzer::AnalyzerError;
use quent_events::Event;
use quent_query_engine_analyzer::{EngineEntity, QueryEngineModel, QueryEntity, ui::UiAnalyzer};
use quent_query_engine_ui::{
    self as ui, EngineContexts, OperatorFilter, QueryFilter, ServerContract,
};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_store::SimulatorEvent;
use quent_ui::{
    entities::request::EntityListRequest,
    timeline::{
        categorical::CategoricalTimelineRequest,
        request::{BulkTimelineRequest, SingleTimelineRequest},
    },
};
use serde::Serialize;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

/// An in-browser implementation of the simulator's typed analyzer API.
#[wasm_bindgen]
pub struct BrowserServer {
    analyzer: SimulatorUiAnalyzer,
    engine_id: Uuid,
    context_ids: Vec<Uuid>,
}

#[wasm_bindgen]
impl BrowserServer {
    /// Load a Postcard-encoded simulator event recording.
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<BrowserServer, JsValue> {
        let bundle = quent_browser_bundle::decode(bytes)
            .map_err(|error| js_error(format!("unable to decode browser bundle: {error}")))?;
        if bundle.metadata.model_name != "quent-simulator" {
            return Err(js_error(format!(
                "browser bundle model mismatch: expected quent-simulator, got {}",
                bundle.metadata.model_name
            )));
        }
        if bundle.metadata.codec != "postcard" {
            return Err(js_error(format!(
                "unsupported simulator event codec: {}",
                bundle.metadata.codec
            )));
        }
        let events: Vec<Event<SimulatorEvent>> = postcard::from_bytes(bundle.payload)
            .map_err(|error| js_error(format!("unable to decode simulator events: {error}")))?;
        let engine_id = events
            .iter()
            .find_map(|event| matches!(event.data, SimulatorEvent::Engine(_)).then_some(event.id))
            .ok_or_else(|| js_error("simulator demo contains no engine events"))?;
        let analyzer = SimulatorUiAnalyzer::try_new(engine_id, events.into_iter())
            .map_err(|error| js_error(format!("unable to analyze simulator demo: {error}")))?;
        if engine_id != bundle.metadata.engine_id {
            return Err(js_error(
                "browser bundle engine ID does not match its events",
            ));
        }
        Ok(Self {
            analyzer,
            engine_id,
            context_ids: bundle.metadata.context_ids,
        })
    }

    #[wasm_bindgen(js_name = listEngines)]
    pub async fn list_engines_js(&self) -> Result<String, JsValue> {
        json_result(ServerContract::list_engines(self, true).await)
    }

    #[wasm_bindgen(js_name = engineContexts)]
    pub async fn engine_contexts_js(&self, engine_id: &str) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        json_result(ServerContract::engine_contexts(self, engine_id).await)
    }

    #[wasm_bindgen(js_name = queries)]
    pub async fn queries_js(&self, engine_id: &str) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        json_result(ServerContract::queries(self, engine_id).await)
    }

    #[wasm_bindgen(js_name = query)]
    pub async fn query_js(&self, engine_id: &str, query_id: &str) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        let query_id = parse_uuid(query_id).map_err(api_error)?;
        json_result(ServerContract::query(self, engine_id, query_id).await)
    }

    #[wasm_bindgen(js_name = singleTimeline)]
    pub async fn single_timeline_js(
        &self,
        engine_id: &str,
        request: &str,
    ) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        let request = parse(request).map_err(api_error)?;
        json_result(ServerContract::single_timeline(self, engine_id, request).await)
    }

    #[wasm_bindgen(js_name = bulkTimelines)]
    pub async fn bulk_timelines_js(
        &self,
        engine_id: &str,
        request: &str,
    ) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        let request = parse(request).map_err(api_error)?;
        json_result(ServerContract::bulk_timelines(self, engine_id, request).await)
    }

    #[wasm_bindgen(js_name = dataFlowTimeline)]
    pub async fn data_flow_timeline_js(
        &self,
        engine_id: &str,
        request: &str,
    ) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        let request = parse(request).map_err(api_error)?;
        json_result(ServerContract::data_flow_timeline(self, engine_id, request).await)
    }

    #[wasm_bindgen(js_name = entities)]
    pub async fn entities_js(&self, engine_id: &str, request: &str) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        let request = parse(request).map_err(api_error)?;
        json_result(ServerContract::entities(self, engine_id, request).await)
    }
}

impl BrowserServer {
    fn analyzer(&self, engine_id: Uuid) -> Result<&SimulatorUiAnalyzer, ApiError> {
        (engine_id == self.engine_id)
            .then_some(&self.analyzer)
            .ok_or(ApiError::NotFound)
    }
}

#[async_trait::async_trait]
impl ServerContract for BrowserServer {
    type Error = ApiError;

    async fn list_engines(&self, with_metadata: bool) -> Result<ui::EngineListResponse, ApiError> {
        let items = if with_metadata {
            vec![self.analyzer.query_engine_model().engine()?.to_ui()?]
        } else {
            vec![ui::Engine::new(self.engine_id)]
        };
        Ok(ui::EngineListResponse {
            items,
            initial_group_by_attribute: SimulatorUiAnalyzer::engine_initial_group_by_attribute(),
        })
    }

    async fn engine(&self, engine_id: Uuid) -> Result<ui::Engine, ApiError> {
        Ok(self
            .analyzer(engine_id)?
            .query_engine_model()
            .engine()?
            .to_ui()?)
    }

    async fn engine_contexts(&self, engine_id: Uuid) -> Result<EngineContexts, ApiError> {
        self.analyzer(engine_id)?;
        Ok(EngineContexts {
            engine_id,
            context_ids: self.context_ids.clone(),
        })
    }

    async fn queries(&self, engine_id: Uuid) -> Result<ui::QueryListResponse, ApiError> {
        let analyzer = self.analyzer(engine_id)?;
        let items = analyzer
            .query_engine_model()
            .queries()
            .map(QueryEntity::to_ui)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ui::QueryListResponse {
            items,
            initial_group_by_attribute: analyzer.query_initial_group_by_attribute(),
        })
    }

    async fn query(&self, engine_id: Uuid, query_id: Uuid) -> Result<ui::QueryBundle, ApiError> {
        self.analyzer(engine_id)?
            .query_bundle(query_id)
            .map_err(Into::into)
    }

    async fn single_timeline(
        &self,
        engine_id: Uuid,
        request: SingleTimelineRequest<QueryFilter, OperatorFilter>,
    ) -> Result<quent_ui::timeline::response::SingleTimelineResponse, ApiError> {
        self.analyzer(engine_id)?
            .single_resource_timeline(request)
            .map_err(Into::into)
    }

    async fn bulk_timelines(
        &self,
        engine_id: Uuid,
        request: BulkTimelineRequest<QueryFilter, OperatorFilter>,
    ) -> Result<quent_ui::timeline::response::BulkTimelinesResponse, ApiError> {
        self.analyzer(engine_id)?
            .bulk_resource_timeline(request)
            .map_err(Into::into)
    }

    async fn data_flow_timeline(
        &self,
        engine_id: Uuid,
        request: CategoricalTimelineRequest<QueryFilter>,
    ) -> Result<ui::DataFlowTimelineBinned, ApiError> {
        self.analyzer(engine_id)?
            .data_flow_timeline(request)
            .map_err(analyzer_error)
    }

    async fn entities(
        &self,
        engine_id: Uuid,
        request: EntityListRequest<QueryFilter, OperatorFilter>,
    ) -> Result<ui::EntityListResponse, ApiError> {
        self.analyzer(engine_id)?
            .list_entities(request)
            .map_err(Into::into)
    }
}

pub enum ApiError {
    InvalidRequest(String),
    NotFound,
    Unsupported(String),
    Other(String),
}

impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(error: E) -> Self {
        Self::Other(error.to_string())
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|error| ApiError::InvalidRequest(error.to_string()))
}

fn parse<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, ApiError> {
    serde_json::from_str(body).map_err(|error| ApiError::InvalidRequest(error.to_string()))
}

fn json_result<T: Serialize>(result: Result<T, ApiError>) -> Result<String, JsValue> {
    serde_json::to_string(&result.map_err(api_error)?).map_err(|error| js_error(error.to_string()))
}

fn api_error(error: ApiError) -> JsValue {
    match error {
        ApiError::InvalidRequest(message) => js_error(format!("QUENT_INVALID_REQUEST:{message}")),
        ApiError::NotFound => js_error("QUENT_NOT_FOUND:not found"),
        ApiError::Unsupported(message) => js_error(format!("QUENT_UNSUPPORTED:{message}")),
        ApiError::Other(message) => js_error(message),
    }
}

fn analyzer_error(error: AnalyzerError) -> ApiError {
    match error {
        error @ AnalyzerError::Unsupported => ApiError::Unsupported(error.to_string()),
        error => ApiError::Other(error.to_string()),
    }
}

fn js_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

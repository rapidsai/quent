// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Browser-hosted API facade for the simulator analyzer.

use quent_events::Event;
use quent_query_engine_analyzer::{
    EngineEntity, QueryEngineModel, QueryEntity, QueryGroupEntity, ui::UiAnalyzer,
};
use quent_query_engine_ui::{
    self as ui, EngineContexts, OperatorFilter, QueryFilter, ServerContract,
};
use quent_simulator_analyzer::SimulatorUiAnalyzer;
use quent_simulator_instrumentation::SimulatorEvent;
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
pub struct DemoServer {
    analyzer: SimulatorUiAnalyzer,
    engine_id: Uuid,
}

#[wasm_bindgen]
impl DemoServer {
    /// Load a Postcard-encoded simulator event recording.
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Result<DemoServer, JsValue> {
        let events: Vec<Event<SimulatorEvent>> = postcard::from_bytes(bytes)
            .map_err(|error| js_error(format!("unable to decode simulator events: {error}")))?;
        let engine_id = events
            .iter()
            .find_map(|event| matches!(event.data, SimulatorEvent::Engine(_)).then_some(event.id))
            .ok_or_else(|| js_error("simulator demo contains no engine events"))?;
        let analyzer = SimulatorUiAnalyzer::try_new(engine_id, events.into_iter())
            .map_err(|error| js_error(format!("unable to analyze simulator demo: {error}")))?;
        Ok(Self {
            analyzer,
            engine_id,
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

    #[wasm_bindgen(js_name = queryGroups)]
    pub async fn query_groups_js(&self, engine_id: &str) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        json_result(ServerContract::query_groups(self, engine_id).await)
    }

    #[wasm_bindgen(js_name = queries)]
    pub async fn queries_js(
        &self,
        engine_id: &str,
        query_group_id: &str,
    ) -> Result<String, JsValue> {
        let engine_id = parse_uuid(engine_id).map_err(api_error)?;
        let query_group_id = parse_uuid(query_group_id).map_err(api_error)?;
        json_result(ServerContract::queries(self, engine_id, query_group_id).await)
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

impl DemoServer {
    fn analyzer(&self, engine_id: Uuid) -> Result<&SimulatorUiAnalyzer, ApiError> {
        (engine_id == self.engine_id)
            .then_some(&self.analyzer)
            .ok_or(ApiError::NotFound)
    }
}

#[async_trait::async_trait]
impl ServerContract for DemoServer {
    type Error = ApiError;
    type EntityRef = <SimulatorUiAnalyzer as UiAnalyzer>::EntityRef;

    async fn list_engines(&self, with_metadata: bool) -> Result<Vec<ui::Engine>, ApiError> {
        if with_metadata {
            Ok(vec![self.analyzer.query_engine_model().engine()?.to_ui()?])
        } else {
            Ok(vec![ui::Engine::new(self.engine_id)])
        }
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
            context_resources: Default::default(),
        })
    }

    async fn query_groups(&self, engine_id: Uuid) -> Result<Vec<ui::QueryGroup>, ApiError> {
        Ok(self
            .analyzer(engine_id)?
            .query_engine_model()
            .query_groups()
            .map(QueryGroupEntity::to_ui)
            .collect())
    }

    async fn queries(
        &self,
        engine_id: Uuid,
        query_group_id: Uuid,
    ) -> Result<Vec<ui::Query>, ApiError> {
        self.analyzer(engine_id)?
            .query_engine_model()
            .queries()
            .filter(|query| query.query_group_id() == Some(query_group_id))
            .map(QueryEntity::to_ui)
            .collect::<Result<_, _>>()
            .map_err(Into::into)
    }

    async fn query(
        &self,
        engine_id: Uuid,
        query_id: Uuid,
    ) -> Result<ui::QueryBundle<Self::EntityRef>, ApiError> {
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
            .map_err(Into::into)
    }

    async fn entities(
        &self,
        engine_id: Uuid,
        request: EntityListRequest<QueryFilter, OperatorFilter>,
    ) -> Result<quent_ui::entities::response::EntityListResponse, ApiError> {
        self.analyzer(engine_id)?
            .list_entities(request)
            .map_err(Into::into)
    }
}

pub enum ApiError {
    NotFound,
    Other(String),
}

impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(error: E) -> Self {
        Self::Other(error.to_string())
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(Into::into)
}

fn parse<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, ApiError> {
    serde_json::from_str(body).map_err(Into::into)
}

fn json_result<T: Serialize>(result: Result<T, ApiError>) -> Result<String, JsValue> {
    serde_json::to_string(&result.map_err(api_error)?).map_err(|error| js_error(error.to_string()))
}

fn api_error(error: ApiError) -> JsValue {
    match error {
        ApiError::NotFound => js_error("not found"),
        ApiError::Other(message) => js_error(message),
    }
}

fn js_error(message: impl AsRef<str>) -> JsValue {
    JsValue::from_str(message.as_ref())
}

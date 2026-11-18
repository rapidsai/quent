// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

//! Shared service and WASM bindings for browser-hosted query-engine analyzers.

use quent_analyzer::{AnalyzerError, AnalyzerResult};
use quent_query_engine_analyzer::{EngineEntity, QueryEngineModel, QueryEntity, ui::UiAnalyzer};
use quent_query_engine_ui::{
    self as ui, EngineContexts, OperatorFilter, QueryFilter, ServerContract,
};
use quent_ui::{
    entities::request::EntityListRequest,
    timeline::{
        categorical::CategoricalTimelineRequest,
        request::{BulkTimelineRequest, SingleTimelineRequest},
    },
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

/// A single-engine analyzer exposed through the query-engine browser contract.
pub struct BrowserService<A> {
    analyzer: A,
    engine_id: Uuid,
    context_ids: Vec<Uuid>,
}

impl<A> BrowserService<A> {
    /// Create a browser service from a model-specific analyzer.
    pub fn new(analyzer: A, engine_id: Uuid, context_ids: Vec<Uuid>) -> Self {
        Self {
            analyzer,
            engine_id,
            context_ids,
        }
    }

    /// Access the model-specific analyzer.
    pub fn analyzer(&self) -> &A {
        &self.analyzer
    }

    fn analyzer_for(&self, engine_id: Uuid) -> Result<&A, BrowserError> {
        (engine_id == self.engine_id)
            .then_some(&self.analyzer)
            .ok_or(BrowserError::NotFound)
    }
}

impl<A> BrowserService<A>
where
    A: UiAnalyzer + Sync,
{
    /// Serialize the engine catalog response.
    pub async fn list_engines_json(&self) -> Result<String, BrowserError> {
        serialize(ServerContract::list_engines(self, true).await)
    }

    /// Parse an engine ID and serialize its contributing contexts.
    pub async fn engine_contexts_json(&self, engine_id: &str) -> Result<String, BrowserError> {
        serialize(ServerContract::engine_contexts(self, parse_uuid(engine_id)?).await)
    }

    /// Parse an engine ID and serialize its query catalog.
    pub async fn queries_json(&self, engine_id: &str) -> Result<String, BrowserError> {
        serialize(ServerContract::queries(self, parse_uuid(engine_id)?).await)
    }

    /// Parse IDs and serialize one query bundle.
    pub async fn query_json(
        &self,
        engine_id: &str,
        query_id: &str,
    ) -> Result<String, BrowserError> {
        serialize(ServerContract::query(self, parse_uuid(engine_id)?, parse_uuid(query_id)?).await)
    }

    /// Parse and execute one single-timeline request.
    pub async fn single_timeline_json(
        &self,
        engine_id: &str,
        request: &str,
    ) -> Result<String, BrowserError> {
        serialize(
            ServerContract::single_timeline(self, parse_uuid(engine_id)?, parse_json(request)?)
                .await,
        )
    }

    /// Parse and execute one bulk-timelines request.
    pub async fn bulk_timelines_json(
        &self,
        engine_id: &str,
        request: &str,
    ) -> Result<String, BrowserError> {
        serialize(
            ServerContract::bulk_timelines(self, parse_uuid(engine_id)?, parse_json(request)?)
                .await,
        )
    }

    /// Parse and execute one data-flow timeline request.
    pub async fn data_flow_timeline_json(
        &self,
        engine_id: &str,
        request: &str,
    ) -> Result<String, BrowserError> {
        serialize(
            ServerContract::data_flow_timeline(self, parse_uuid(engine_id)?, parse_json(request)?)
                .await,
        )
    }

    /// Parse and execute one entity-list request.
    pub async fn entities_json(
        &self,
        engine_id: &str,
        request: &str,
    ) -> Result<String, BrowserError> {
        serialize(
            ServerContract::entities(self, parse_uuid(engine_id)?, parse_json(request)?).await,
        )
    }
}

#[async_trait::async_trait]
impl<A> ServerContract for BrowserService<A>
where
    A: UiAnalyzer + Sync,
{
    type Error = BrowserError;

    async fn list_engines(
        &self,
        with_metadata: bool,
    ) -> Result<ui::EngineListResponse, BrowserError> {
        let items = if with_metadata {
            vec![self.analyzer.query_engine_model().engine()?.to_ui()?]
        } else {
            vec![ui::Engine::new(self.engine_id)]
        };
        Ok(ui::EngineListResponse {
            items,
            initial_group_by_attribute: A::engine_initial_group_by_attribute(),
        })
    }

    async fn engine(&self, engine_id: Uuid) -> Result<ui::Engine, BrowserError> {
        Ok(self
            .analyzer_for(engine_id)?
            .query_engine_model()
            .engine()?
            .to_ui()?)
    }

    async fn engine_contexts(&self, engine_id: Uuid) -> Result<EngineContexts, BrowserError> {
        self.analyzer_for(engine_id)?;
        Ok(EngineContexts {
            engine_id,
            context_ids: self.context_ids.clone(),
        })
    }

    async fn queries(&self, engine_id: Uuid) -> Result<ui::QueryListResponse, BrowserError> {
        let analyzer = self.analyzer_for(engine_id)?;
        let items = analyzer
            .query_engine_model()
            .queries()
            .map(QueryEntity::to_ui)
            .collect::<AnalyzerResult<Vec<_>>>()?;
        Ok(ui::QueryListResponse {
            items,
            initial_group_by_attribute: analyzer.query_initial_group_by_attribute(),
        })
    }

    async fn query(
        &self,
        engine_id: Uuid,
        query_id: Uuid,
    ) -> Result<ui::QueryBundle, BrowserError> {
        Ok(self.analyzer_for(engine_id)?.query_bundle(query_id)?)
    }

    async fn single_timeline(
        &self,
        engine_id: Uuid,
        request: SingleTimelineRequest<QueryFilter, OperatorFilter>,
    ) -> Result<quent_ui::timeline::response::SingleTimelineResponse, BrowserError> {
        Ok(self
            .analyzer_for(engine_id)?
            .single_resource_timeline(request)?)
    }

    async fn bulk_timelines(
        &self,
        engine_id: Uuid,
        request: BulkTimelineRequest<QueryFilter, OperatorFilter>,
    ) -> Result<quent_ui::timeline::response::BulkTimelinesResponse, BrowserError> {
        Ok(self
            .analyzer_for(engine_id)?
            .bulk_resource_timeline(request)?)
    }

    async fn data_flow_timeline(
        &self,
        engine_id: Uuid,
        request: CategoricalTimelineRequest<QueryFilter>,
    ) -> Result<ui::DataFlowTimelineBinned, BrowserError> {
        Ok(self.analyzer_for(engine_id)?.data_flow_timeline(request)?)
    }

    async fn entities(
        &self,
        engine_id: Uuid,
        request: EntityListRequest<QueryFilter, OperatorFilter>,
    ) -> Result<ui::EntityListResponse, BrowserError> {
        Ok(self.analyzer_for(engine_id)?.list_entities(request)?)
    }
}

/// Stable error protocol shared by the WASM worker and model-specific analyzers.
#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("QUENT_INVALID_REQUEST:{0}")]
    InvalidRequest(String),
    #[error("QUENT_NOT_FOUND:not found")]
    NotFound,
    #[error("QUENT_UNSUPPORTED:{0}")]
    Unsupported(String),
    #[error("{0}")]
    Failure(String),
}

impl BrowserError {
    /// Create an unclassified analyzer or bundle-loading failure.
    pub fn failure(error: impl std::fmt::Display) -> Self {
        Self::Failure(error.to_string())
    }
}

impl From<AnalyzerError> for BrowserError {
    fn from(error: AnalyzerError) -> Self {
        match error {
            error @ AnalyzerError::Unsupported => Self::Unsupported(error.to_string()),
            error => Self::Failure(error.to_string()),
        }
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, BrowserError> {
    Uuid::parse_str(value).map_err(|error| BrowserError::InvalidRequest(error.to_string()))
}

fn parse_json<T: DeserializeOwned>(body: &str) -> Result<T, BrowserError> {
    serde_json::from_str(body).map_err(|error| BrowserError::InvalidRequest(error.to_string()))
}

fn serialize<T: Serialize>(result: Result<T, BrowserError>) -> Result<String, BrowserError> {
    serde_json::to_string(&result?).map_err(BrowserError::failure)
}

/// Export the standard `BrowserServer` JavaScript class for a model-specific loader.
///
/// The loader must have the signature
/// `fn(&[u8]) -> Result<BrowserService<Analyzer>, BrowserError>`.
#[macro_export]
macro_rules! export_wasm_browser_server {
    ($analyzer:ty, $loader:path) => {
        #[wasm_bindgen::prelude::wasm_bindgen]
        pub struct BrowserServer {
            inner: $crate::BrowserService<$analyzer>,
        }

        #[wasm_bindgen::prelude::wasm_bindgen]
        impl BrowserServer {
            #[wasm_bindgen::prelude::wasm_bindgen(constructor)]
            pub fn new(bytes: &[u8]) -> Result<Self, wasm_bindgen::JsValue> {
                $loader(bytes)
                    .map(|inner| Self { inner })
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = listEngines)]
            pub async fn list_engines_js(&self) -> Result<String, wasm_bindgen::JsValue> {
                self.inner
                    .list_engines_json()
                    .await
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = engineContexts)]
            pub async fn engine_contexts_js(
                &self,
                engine_id: &str,
            ) -> Result<String, wasm_bindgen::JsValue> {
                self.inner
                    .engine_contexts_json(engine_id)
                    .await
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = queries)]
            pub async fn queries_js(
                &self,
                engine_id: &str,
            ) -> Result<String, wasm_bindgen::JsValue> {
                self.inner
                    .queries_json(engine_id)
                    .await
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = query)]
            pub async fn query_js(
                &self,
                engine_id: &str,
                query_id: &str,
            ) -> Result<String, wasm_bindgen::JsValue> {
                self.inner
                    .query_json(engine_id, query_id)
                    .await
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = singleTimeline)]
            pub async fn single_timeline_js(
                &self,
                engine_id: &str,
                request: &str,
            ) -> Result<String, wasm_bindgen::JsValue> {
                self.inner
                    .single_timeline_json(engine_id, request)
                    .await
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = bulkTimelines)]
            pub async fn bulk_timelines_js(
                &self,
                engine_id: &str,
                request: &str,
            ) -> Result<String, wasm_bindgen::JsValue> {
                self.inner
                    .bulk_timelines_json(engine_id, request)
                    .await
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = dataFlowTimeline)]
            pub async fn data_flow_timeline_js(
                &self,
                engine_id: &str,
                request: &str,
            ) -> Result<String, wasm_bindgen::JsValue> {
                self.inner
                    .data_flow_timeline_json(engine_id, request)
                    .await
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = entities)]
            pub async fn entities_js(
                &self,
                engine_id: &str,
                request: &str,
            ) -> Result<String, wasm_bindgen::JsValue> {
                self.inner
                    .entities_json(engine_id, request)
                    .await
                    .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
            }
        }
    };
}

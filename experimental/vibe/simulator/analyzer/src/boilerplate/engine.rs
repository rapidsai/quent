// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use schema::entity_events::engine::{EngineEventStorage, EngineEvents};

use super::*;

pub type Engine = StoredEntity<schema::Engine, EngineEventStorage>;

impl RefTreeEntity for Engine {
    fn parent_id(&self) -> Option<Uuid> {
        None
    }
}

impl EngineEntity for Engine {
    fn to_ui(&self) -> AnalyzerResult<query_engine_ui::Engine> {
        let start = self.earliest_timestamp();
        let duration_s = self
            .exit()
            .map(|_| try_to_secs_relative(self.latest_timestamp(), start))
            .transpose()?;
        let init = self.init().map(|event| &event.data);
        Ok(query_engine_ui::Engine {
            id: self.id(),
            start_time_unix_ns: Some(start),
            duration_s,
            instance_name: init.and_then(|data| data.instance_name.clone()),
            implementation: init.map(|data| {
                let implementation = &data.implementation;
                query_engine_ui::EngineImplementationAttributes {
                    name: implementation.name.clone(),
                    version: implementation.version.clone(),
                    custom_attributes: implementation.custom_attributes.0.clone(),
                }
            }),
        })
    }
}

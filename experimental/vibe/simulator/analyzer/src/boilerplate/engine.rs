// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use quent_simulator_store::engine::EngineEvents;

use super::*;

pub type Engine = EntityAnalysis<schema::Engine>;

pub(crate) fn engine_custom_attributes(
    configuration: &schema::EngineConfiguration,
) -> Vec<DynamicAttribute> {
    vec![
        DynamicAttribute::u64("workers", configuration.workers),
        DynamicAttribute::u64("threads_per_worker", configuration.threads_per_worker),
        DynamicAttribute::u64("gpus_per_worker", configuration.gpus_per_worker),
        DynamicAttribute::u64("num_workloads", configuration.num_workloads),
        DynamicAttribute::u64("num_queries", configuration.num_queries),
    ]
}

impl RefTreeEntity for Engine {
    fn parent_id(&self) -> Option<Uuid> {
        None
    }
}

impl EngineEntity for Engine {
    fn to_ui(&self) -> AnalyzerResult<query_engine_ui::Engine> {
        let init = self.init();
        let start = self.earliest_timestamp();
        let duration_s = self
            .exit()
            .is_some()
            .then(|| try_to_secs_relative(self.latest_timestamp(), start))
            .transpose()?;
        Ok(query_engine_ui::Engine {
            id: self.id(),
            start_time_unix_ns: Some(start),
            duration_s,
            instance_name: init.and_then(|event| event.instance_name().clone()),
            custom_attributes: init
                .map(|event| engine_custom_attributes(event.configuration()))
                .unwrap_or_default(),
            implementation: init.map(|event| {
                let implementation = event.implementation();
                query_engine_ui::EngineImplementationAttributes {
                    name: implementation.name.clone(),
                    version: implementation.version.clone(),
                    custom_attributes: implementation.custom_attributes.0.clone(),
                }
            }),
        })
    }
}

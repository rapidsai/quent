use quent::{Context, ExporterOptions};
use quent_exporter_collector::CollectorExporterOptions;

pub mod attributes;
pub mod engine;
pub mod operator;
pub mod plan;
pub mod query;
pub mod query_group;
pub mod uuid;
pub mod worker;

pub struct QuentContext {
    context: Context,
    engine_observer: Option<engine::EngineObserver>,
    query_group_observer: Option<query_group::QueryGroupObserver>,
    worker_observer: Option<worker::WorkerObserver>,
    query_observer: Option<query::QueryObserver>,
    plan_observer: Option<plan::PlanObserver>,
    operator_observer: Option<operator::OperatorObserver>,
}

impl QuentContext {
    pub fn initialize_with_collector_exporter(
        engine_id: uuid::ffi::UUID,
        maybe_collector_address: String,
    ) -> Result<Box<Self>, Box<dyn std::error::Error>> {
        Ok(Box::new(Self {
            context: Context::try_new(
                ExporterOptions::Collector(CollectorExporterOptions {
                    address: (!maybe_collector_address.is_empty())
                        .then_some(maybe_collector_address),
                }),
                engine_id.into(),
            )?,
            engine_observer: None,
            query_group_observer: None,
            worker_observer: None,
            query_observer: None,
            plan_observer: None,
            operator_observer: None,
        }))
    }

    pub fn initialize_with_ndjson_exporter(
        engine_id: uuid::ffi::UUID,
    ) -> Result<Box<Self>, Box<dyn std::error::Error>> {
        Ok(Box::new(Self {
            context: Context::try_new(ExporterOptions::Ndjson, engine_id.into())?,
            engine_observer: None,
            query_group_observer: None,
            worker_observer: None,
            query_observer: None,
            plan_observer: None,
            operator_observer: None,
        }))
    }

    pub fn engine_observer(&mut self) -> &engine::EngineObserver {
        self.engine_observer
            .get_or_insert_with(|| engine::EngineObserver {
                inner: self.context.engine_observer(),
            })
    }

    pub fn query_group_observer(&mut self) -> &query_group::QueryGroupObserver {
        self.query_group_observer
            .get_or_insert_with(|| query_group::QueryGroupObserver {
                inner: self.context.query_group_observer(),
            })
    }

    pub fn worker_observer(&mut self) -> &worker::WorkerObserver {
        self.worker_observer
            .get_or_insert_with(|| worker::WorkerObserver {
                inner: self.context.worker_observer(),
            })
    }

    pub fn query_observer(&mut self) -> &query::QueryObserver {
        self.query_observer
            .get_or_insert_with(|| query::QueryObserver {
                inner: self.context.query_observer(),
            })
    }

    pub fn plan_observer(&mut self) -> &plan::PlanObserver {
        self.plan_observer
            .get_or_insert_with(|| plan::PlanObserver {
                inner: self.context.plan_observer(),
            })
    }

    pub fn operator_observer(&mut self) -> &operator::OperatorObserver {
        self.operator_observer
            .get_or_insert_with(|| operator::OperatorObserver {
                inner: self.context.operator_observer(),
            })
    }
}

#[cxx::bridge(namespace = "quent")]
mod ffi {
    #[namespace = "uuid"]
    extern "C++" {
        include!("quent-cpp/src/uuid.rs.h");
        type UUID = crate::uuid::ffi::UUID;
    }

    // These observers are defined in separate bridges with their own namespaces.
    // By declaring them as extern "C++" here and implementing ExternType trait
    // in their respective files, we can reference them across bridges.
    // See: https://github.com/dtolnay/cxx/issues/942

    #[namespace = "quent::engine"]
    extern "C++" {
        include!("quent-cpp/src/engine.rs.h");
        type engine_observer = crate::engine::EngineObserver;
    }

    #[namespace = "quent::query_group"]
    extern "C++" {
        include!("quent-cpp/src/query_group.rs.h");
        type query_group_observer = crate::query_group::QueryGroupObserver;
    }

    #[namespace = "quent::worker"]
    extern "C++" {
        include!("quent-cpp/src/worker.rs.h");
        type worker_observer = crate::worker::WorkerObserver;
    }

    #[namespace = "quent::query"]
    extern "C++" {
        include!("quent-cpp/src/query.rs.h");
        type query_observer = crate::query::QueryObserver;
    }

    #[namespace = "quent::plan"]
    extern "C++" {
        include!("quent-cpp/src/plan.rs.h");
        type plan_observer = crate::plan::PlanObserver;
    }

    #[namespace = "quent::plan_operator"]
    extern "C++" {
        include!("quent-cpp/src/operator.rs.h");
        type operator_observer = crate::operator::OperatorObserver;
    }

    extern "Rust" {
        #[cxx_name = "quent_context"]
        type QuentContext;

        #[Self = "QuentContext"]
        fn initialize_with_collector_exporter(
            engine_id: UUID,
            maybe_collector_address: String,
        ) -> Result<Box<QuentContext>>;

        #[Self = "QuentContext"]
        fn initialize_with_ndjson_exporter(engine_id: UUID) -> Result<Box<QuentContext>>;

        fn engine_observer(self: &mut QuentContext) -> &engine_observer;
        fn query_group_observer(self: &mut QuentContext) -> &query_group_observer;
        fn worker_observer(self: &mut QuentContext) -> &worker_observer;
        fn query_observer(self: &mut QuentContext) -> &query_observer;
        fn plan_observer(self: &mut QuentContext) -> &plan_observer;
        fn operator_observer(self: &mut QuentContext) -> &operator_observer;
    }
}

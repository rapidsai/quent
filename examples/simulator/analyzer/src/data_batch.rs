use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, FsmTypeDecl, FsmTypeDeclaration, FsmUsages, Transition},
    resource::{CapacityValue, Usage, Using},
};
use quent_attributes::Attribute;
use quent_events::Event;
use quent_simulator_events::data_batch::{
    DataBatchEvent, InMemory, LoadingToGpu, LoadingToMemory, OnDisk, OnGpu, SpillingToDisk,
    SpillingToMemory,
};
use quent_time::{
    TimeOrderedCollector, TimeUnixNanoSec, Timestamp, span::SpanUnixNanoSec, to_secs_relative,
};
use quent_ui::{FiniteStateMachine, FsmTransition, FsmUsage};
use smallvec::{SmallVec, smallvec};
use uuid::Uuid;

#[derive(Debug)]
pub enum DataBatchTransitionData {
    OnDisk(OnDisk),
    LoadingToMemory(LoadingToMemory),
    InMemory(InMemory),
    LoadingToGpu(LoadingToGpu),
    OnGpu(OnGpu),
    SpillingToMemory(SpillingToMemory),
    SpillingToDisk(SpillingToDisk),
    Exit,
}

#[derive(Debug)]
pub struct DataBatchUsage {
    pub resource_id: Uuid,
    pub capacities: SmallVec<[CapacityValue; 1]>,
}

pub struct DataBatchUsageWithSpan<'a> {
    batch_id: Uuid,
    usage: &'a DataBatchUsage,
    span: SpanUnixNanoSec,
}

impl<'a> Usage<'a> for DataBatchUsageWithSpan<'a> {
    fn entity_id(&self) -> Uuid {
        self.batch_id
    }
    fn resource_id(&self) -> Uuid {
        self.usage.resource_id
    }
    fn capacities(&self) -> impl Iterator<Item = &'a CapacityValue> {
        self.usage.capacities.iter()
    }
    fn span(&self) -> SpanUnixNanoSec {
        self.span
    }
}

#[derive(Debug)]
pub struct DataBatchTransition {
    timestamp: TimeUnixNanoSec,
    data: DataBatchTransitionData,
    usages: SmallVec<[DataBatchUsage; 1]>,
}

impl Timestamp for DataBatchTransition {
    fn timestamp(&self) -> TimeUnixNanoSec {
        self.timestamp
    }
}

impl Transition for DataBatchTransition {
    fn name(&self) -> &str {
        match &self.data {
            DataBatchTransitionData::OnDisk(_) => "on_disk",
            DataBatchTransitionData::LoadingToMemory(_) => "loading_to_memory",
            DataBatchTransitionData::InMemory(_) => "in_memory",
            DataBatchTransitionData::LoadingToGpu(_) => "loading_to_gpu",
            DataBatchTransitionData::OnGpu(_) => "on_gpu",
            DataBatchTransitionData::SpillingToMemory(_) => "spilling_to_memory",
            DataBatchTransitionData::SpillingToDisk(_) => "spilling_to_disk",
            DataBatchTransitionData::Exit => "exit",
        }
    }

    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        std::iter::empty()
    }
}

fn create_usages(data: &DataBatchTransitionData) -> SmallVec<[DataBatchUsage; 1]> {
    match data {
        DataBatchTransitionData::OnDisk(data) => smallvec![DataBatchUsage {
            resource_id: data.use_filesystem,
            capacities: smallvec![CapacityValue::new("bytes", data.use_filesystem_bytes)],
        }],
        DataBatchTransitionData::LoadingToMemory(data) => smallvec![DataBatchUsage {
            resource_id: data.use_fs_to_mem,
            capacities: smallvec![CapacityValue::new("bytes", data.use_fs_to_mem_bytes)],
        }],
        DataBatchTransitionData::InMemory(data) => smallvec![DataBatchUsage {
            resource_id: data.use_memory,
            capacities: smallvec![CapacityValue::new("bytes", data.use_memory_bytes)],
        }],
        DataBatchTransitionData::LoadingToGpu(data) => smallvec![DataBatchUsage {
            resource_id: data.use_mem_to_gpu,
            capacities: smallvec![CapacityValue::new("bytes", data.use_mem_to_gpu_bytes)],
        }],
        DataBatchTransitionData::OnGpu(data) => smallvec![DataBatchUsage {
            resource_id: data.use_gpu_memory,
            capacities: smallvec![CapacityValue::new("bytes", data.use_gpu_memory_bytes)],
        }],
        DataBatchTransitionData::SpillingToMemory(data) => smallvec![DataBatchUsage {
            resource_id: data.use_gpu_to_mem,
            capacities: smallvec![CapacityValue::new("bytes", data.use_gpu_to_mem_bytes)],
        }],
        DataBatchTransitionData::SpillingToDisk(data) => smallvec![DataBatchUsage {
            resource_id: data.use_mem_to_fs,
            capacities: smallvec![CapacityValue::new("bytes", data.use_mem_to_fs_bytes)],
        }],
        DataBatchTransitionData::Exit => SmallVec::new(),
    }
}

pub(crate) struct DataBatchBuilder {
    id: Uuid,
    transitions: TimeOrderedCollector<DataBatchTransition>,
}

impl DataBatchBuilder {
    pub(crate) fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "data batch id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                transitions: TimeOrderedCollector::default(),
            })
        }
    }

    pub(crate) fn push(&mut self, event: Event<DataBatchEvent>) {
        let data = match event.data {
            DataBatchEvent::OnDisk(data) => DataBatchTransitionData::OnDisk(data),
            DataBatchEvent::LoadingToMemory(data) => DataBatchTransitionData::LoadingToMemory(data),
            DataBatchEvent::InMemory(data) => DataBatchTransitionData::InMemory(data),
            DataBatchEvent::LoadingToGpu(data) => DataBatchTransitionData::LoadingToGpu(data),
            DataBatchEvent::OnGpu(data) => DataBatchTransitionData::OnGpu(data),
            DataBatchEvent::SpillingToMemory(data) => {
                DataBatchTransitionData::SpillingToMemory(data)
            }
            DataBatchEvent::SpillingToDisk(data) => DataBatchTransitionData::SpillingToDisk(data),
            DataBatchEvent::Exit => DataBatchTransitionData::Exit,
        };
        let usages = create_usages(&data);
        self.transitions.push(DataBatchTransition {
            timestamp: event.timestamp,
            data,
            usages,
        });
    }

    pub(crate) fn try_build(self) -> AnalyzerResult<DataBatch> {
        let transitions: SmallVec<[DataBatchTransition; 4]> = self.transitions.into_inner().into();
        Ok(DataBatch {
            id: self.id,
            transitions,
        })
    }
}

#[derive(Debug)]
pub struct DataBatch {
    id: Uuid,
    transitions: SmallVec<[DataBatchTransition; 4]>,
}

impl DataBatch {
    pub fn operator_id(&self) -> Option<Uuid> {
        self.transitions.first().and_then(|t| match &t.data {
            DataBatchTransitionData::OnDisk(data) => Some(data.operator_id),
            _ => None,
        })
    }

    pub fn try_to_ui_fsm(&self, epoch: TimeUnixNanoSec) -> AnalyzerResult<FiniteStateMachine> {
        let transitions = self
            .transitions
            .iter()
            .map(|t| {
                Ok(FsmTransition {
                    name: t.name().to_string(),
                    usages: t
                        .usages
                        .iter()
                        .map(|u| FsmUsage {
                            resource: u.resource_id,
                            capacities: u
                                .capacities
                                .iter()
                                .map(|c| (c.name.to_string(), c.value))
                                .collect(),
                        })
                        .collect(),
                    timestamp: to_secs_relative(t.timestamp(), epoch),
                })
            })
            .collect::<AnalyzerResult<Vec<_>>>()?;

        Ok(FiniteStateMachine {
            id: self.id,
            type_name: self.type_name().to_string(),
            instance_name: self.instance_name().to_string(),
            transitions,
        })
    }
}

impl Entity for DataBatch {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "data_batch"
    }
    fn instance_name(&self) -> &str {
        ""
    }
}

impl Fsm for DataBatch {
    type TransitionType = DataBatchTransition;
    fn len(&self) -> usize {
        self.transitions.len().saturating_sub(1)
    }
    fn transition(&self, index: usize) -> Option<&Self::TransitionType> {
        self.transitions.get(index)
    }
}

impl<'a> FsmUsages<'a> for DataBatch {
    fn usages_with_state_names(&'a self) -> impl Iterator<Item = (&'a str, impl Usage<'a>)> {
        self.transitions.windows(2).flat_map(move |window| {
            let name = window[0].name();
            let start = window[0].timestamp();
            let end = window[1].timestamp();
            let span = SpanUnixNanoSec::try_new(start, end).unwrap();
            window[0].usages.iter().map(move |u| {
                (
                    name,
                    DataBatchUsageWithSpan {
                        batch_id: self.id,
                        usage: u,
                        span,
                    },
                )
            })
        })
    }
}

impl Using for DataBatch {
    fn usages<'a>(&'a self) -> impl Iterator<Item = impl Usage<'a>> {
        self.transitions.windows(2).flat_map(move |window| {
            let start = window[0].timestamp();
            let end = window[1].timestamp();
            let span = SpanUnixNanoSec::try_new(start, end).unwrap();
            window[0]
                .usages
                .iter()
                .map(move |u| DataBatchUsageWithSpan {
                    batch_id: self.id,
                    usage: u,
                    span,
                })
        })
    }
}

impl FsmTypeDeclaration for DataBatch {
    fn fsm_type_declaration() -> FsmTypeDecl {
        use quent_analyzer::fsm::{FsmStateTypeDecl, FsmTransitionDecl};

        let states = vec![
            FsmStateTypeDecl {
                name: "on_disk".to_string(),
                usages: vec!["filesystem".to_string()],
            },
            FsmStateTypeDecl {
                name: "loading_to_memory".to_string(),
                usages: vec!["fs_to_mem".to_string()],
            },
            FsmStateTypeDecl {
                name: "in_memory".to_string(),
                usages: vec!["memory".to_string()],
            },
            FsmStateTypeDecl {
                name: "loading_to_gpu".to_string(),
                usages: vec!["mem_to_gpu".to_string()],
            },
            FsmStateTypeDecl {
                name: "on_gpu".to_string(),
                usages: vec!["gpu_memory".to_string()],
            },
            FsmStateTypeDecl {
                name: "spilling_to_memory".to_string(),
                usages: vec!["gpu_to_mem".to_string()],
            },
            FsmStateTypeDecl {
                name: "spilling_to_disk".to_string(),
                usages: vec!["mem_to_fs".to_string()],
            },
            FsmStateTypeDecl {
                name: "exit".to_string(),
                usages: vec![],
            },
        ];

        let transitions = vec![
            FsmTransitionDecl::Entry("on_disk".to_string()),
            FsmTransitionDecl::Transition("on_disk".to_string(), "loading_to_memory".to_string()),
            FsmTransitionDecl::Transition("loading_to_memory".to_string(), "in_memory".to_string()),
            FsmTransitionDecl::Transition("in_memory".to_string(), "loading_to_gpu".to_string()),
            FsmTransitionDecl::Transition("in_memory".to_string(), "spilling_to_disk".to_string()),
            FsmTransitionDecl::Transition("in_memory".to_string(), "exit".to_string()),
            FsmTransitionDecl::Transition("loading_to_gpu".to_string(), "on_gpu".to_string()),
            FsmTransitionDecl::Transition("on_gpu".to_string(), "spilling_to_memory".to_string()),
            FsmTransitionDecl::Transition(
                "spilling_to_memory".to_string(),
                "in_memory".to_string(),
            ),
            FsmTransitionDecl::Transition("spilling_to_disk".to_string(), "on_disk".to_string()),
            FsmTransitionDecl::Exit("exit".to_string()),
        ];

        FsmTypeDecl {
            name: "data_batch".to_string(),
            states,
            transitions,
        }
    }
}

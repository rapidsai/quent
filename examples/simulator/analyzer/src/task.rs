use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, OrderedStateTransitionCollector, State, Transition},
    resource::{CapacityValue, Usage, Using},
};
use quent_attributes::Attribute;
use quent_events::Event;
use quent_simulator_events::task::{Init, TaskEvent};
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

pub struct TaskTransition(Event<TaskEvent>);

#[derive(Debug)]
pub struct InitState {
    pub span: SpanUnixNanoSec,
    pub data: Init,
}

#[derive(Debug)]
pub struct QueueingState {
    pub span: SpanUnixNanoSec,
}

#[derive(Debug)]
pub struct ComputingState {
    pub span: SpanUnixNanoSec,
    pub usages: [Usage; 2],
}

#[derive(Debug)]
pub struct LoadingState {
    pub span: SpanUnixNanoSec,
    pub usages: [Usage; 3],
}

#[derive(Debug)]
pub struct AllocatingMemoryState {
    pub span: SpanUnixNanoSec,
    pub usages: [Usage; 1],
}

#[derive(Debug)]
pub struct AllocatingStorageState {
    pub span: SpanUnixNanoSec,
    pub usages: [Usage; 1],
}

#[derive(Debug)]
pub struct SpillingState {
    pub span: SpanUnixNanoSec,
    pub usages: [Usage; 2],
}

#[derive(Debug)]
pub struct SendingState {
    pub span: SpanUnixNanoSec,
    pub usages: [Usage; 2],
}

#[derive(Debug)]
pub struct FinalizingState {
    pub span: SpanUnixNanoSec,
}

#[derive(Debug)]
pub enum TaskState {
    Init(InitState),
    Queueing(QueueingState),
    Computing(ComputingState),
    Loading(LoadingState),
    AllocatingMemory(AllocatingMemoryState),
    AllocatingStorage(AllocatingStorageState),
    Spilling(SpillingState),
    Sending(SendingState),
    Finalizing(FinalizingState),
}

impl Transition for TaskTransition {
    type Target = TaskState;

    fn timestamp(&self) -> TimeUnixNanoSec {
        self.0.timestamp
    }

    // TODO(johanpel): would be nice to have some abstractions to simplify this
    fn try_into_state(self, end: TimeUnixNanoSec) -> AnalyzerResult<Self::Target> {
        let t = self.timestamp();
        Ok(match self.0.data {
            TaskEvent::Init(data) => TaskState::Init(InitState {
                span: SpanUnixNanoSec::try_new(t, end)?,
                data,
            }),
            TaskEvent::Queueing => TaskState::Queueing(QueueingState {
                span: SpanUnixNanoSec::try_new(t, end)?,
            }),
            TaskEvent::Computing(data) => TaskState::Computing(ComputingState {
                span: SpanUnixNanoSec::try_new(t, end)?,
                usages: [
                    Usage::new(data.use_task_thread, [CapacityValue::new("unit", 1)]),
                    Usage::new(
                        data.use_main_memory,
                        [CapacityValue::new("bytes", data.use_main_memory_bytes)],
                    ),
                ],
            }),
            TaskEvent::AllocatingMemory(data) => {
                TaskState::AllocatingMemory(AllocatingMemoryState {
                    span: SpanUnixNanoSec::try_new(t, end)?,
                    usages: [Usage::new(
                        data.use_task_thread,
                        [CapacityValue::new("unit", 1)],
                    )],
                })
            }
            TaskEvent::Loading(data) => TaskState::Loading(LoadingState {
                span: SpanUnixNanoSec::try_new(t, end)?,
                usages: [
                    Usage::new(data.use_task_thread, [CapacityValue::new("unit", 1)]),
                    Usage::new(
                        data.use_fs_to_mem,
                        [CapacityValue::new("bytes", data.use_fs_to_mem_bytes)],
                    ),
                    Usage::new(
                        data.use_main_memory,
                        [CapacityValue::new("bytes", data.use_main_memory_bytes)],
                    ),
                ],
            }),
            TaskEvent::AllocatingStorage(data) => {
                TaskState::AllocatingStorage(AllocatingStorageState {
                    span: SpanUnixNanoSec::try_new(t, end)?,
                    usages: [Usage::new(
                        data.use_task_thread,
                        [CapacityValue::new("unit", 1)],
                    )],
                })
            }
            TaskEvent::Spilling(data) => TaskState::Spilling(SpillingState {
                span: SpanUnixNanoSec::try_new(t, end)?,
                usages: [
                    Usage::new(data.use_task_thread, [CapacityValue::new("unit", 1)]),
                    Usage::new(
                        data.use_mem_to_fs,
                        [CapacityValue::new("bytes", data.use_mem_to_fs_bytes)],
                    ),
                ],
            }),
            TaskEvent::Sending(data) => TaskState::Sending(SendingState {
                span: SpanUnixNanoSec::try_new(t, end)?,
                usages: [
                    Usage::new(data.use_task_thread, [CapacityValue::new("unit", 1)]),
                    Usage::new(
                        data.use_link,
                        [CapacityValue::new("bytes", data.use_link_bytes)],
                    ),
                ],
            }),
            TaskEvent::Finalizing => TaskState::Finalizing(FinalizingState {
                span: SpanUnixNanoSec::try_new(t, end)?,
            }),
            TaskEvent::Exit => Err(AnalyzerError::FsmExitTransitionConversion)?,
        })
    }
}

impl State for TaskState {
    fn name(&self) -> &str {
        match self {
            TaskState::Init(_) => "init",
            TaskState::Queueing(_) => "queueing",
            TaskState::Computing(_) => "computing",
            TaskState::Loading(_) => "loading",
            TaskState::AllocatingMemory(_) => "allocating_memory",
            TaskState::AllocatingStorage(_) => "allocating_storage",
            TaskState::Spilling(_) => "spilling",
            TaskState::Sending(_) => "sending",
            TaskState::Finalizing(_) => "finalizing",
        }
    }
    fn span(&self) -> SpanUnixNanoSec {
        match self {
            TaskState::Init(state) => state.span,
            TaskState::Queueing(state) => state.span,
            TaskState::Computing(state) => state.span,
            TaskState::Loading(state) => state.span,
            TaskState::AllocatingMemory(state) => state.span,
            TaskState::AllocatingStorage(state) => state.span,
            TaskState::Spilling(state) => state.span,
            TaskState::Sending(state) => state.span,
            TaskState::Finalizing(state) => state.span,
        }
    }
    fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        std::iter::empty()
    }
}

impl TaskState {
    pub fn usages(&self) -> impl Iterator<Item = (&Usage, SpanUnixNanoSec)> {
        let (usages, span): (&[Usage], _) = match self {
            TaskState::Init(state) => (&[], state.span),
            TaskState::Queueing(state) => (&[], state.span),
            TaskState::Computing(state) => (&state.usages, state.span),
            TaskState::Loading(state) => (&state.usages, state.span),
            TaskState::AllocatingMemory(state) => (&state.usages, state.span),
            TaskState::AllocatingStorage(state) => (&state.usages, state.span),
            TaskState::Spilling(state) => (&state.usages, state.span),
            TaskState::Sending(state) => (&state.usages, state.span),
            TaskState::Finalizing(state) => (&[], state.span),
        };
        usages.iter().map(move |usage| (usage, span))
    }
}

pub(crate) struct TaskBuilder {
    id: Uuid,
    transitions: OrderedStateTransitionCollector<TaskTransition>,
}

impl TaskBuilder {
    pub(crate) fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "task id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                transitions: OrderedStateTransitionCollector::default(),
            })
        }
    }

    pub(crate) fn push(&mut self, event: Event<TaskEvent>) {
        self.transitions.push(TaskTransition(event))
    }

    pub(crate) fn try_build(self) -> AnalyzerResult<Task> {
        let transitions: Vec<TaskTransition> = self.transitions.try_into()?;
        let len = transitions.len();
        // Collect end timestamps before consuming transitions.
        let end_times: Vec<TimeUnixNanoSec> =
            transitions[1..].iter().map(|t| t.timestamp()).collect();
        let mut sequence = Vec::with_capacity(len.saturating_sub(1));
        for (transition, end) in transitions.into_iter().zip(end_times) {
            sequence.push(transition.try_into_state(end)?);
        }
        Ok(Task {
            id: self.id,
            sequence,
        })
    }
}

#[derive(Debug)]
pub struct Task {
    id: Uuid,
    sequence: Vec<TaskState>,
}

impl Entity for Task {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "task"
    }
    fn instance_name(&self) -> &str {
        self.sequence
            .first()
            .and_then(|maybe_init| match maybe_init {
                TaskState::Init(state) => Some(state.data.instance_name.as_str()),
                _ => None,
            })
            .unwrap_or_default()
    }
}

impl Task {
    pub fn operator_id(&self) -> Option<Uuid> {
        self.sequence.first().and_then(|s| match s {
            TaskState::Init(state) => Some(state.data.operator_id),
            _ => None,
        })
    }
}

impl Fsm for Task {
    type StateType = TaskState;
    fn len(&self) -> usize {
        self.sequence.len()
    }
    fn state(&self, index: usize) -> Option<&Self::StateType> {
        self.sequence.get(index)
    }
    fn states(&self) -> impl ExactSizeIterator<Item = &Self::StateType> {
        self.sequence.iter()
    }
}

impl Using for Task {
    fn usages(&self) -> impl Iterator<Item = (&Usage, SpanUnixNanoSec)> {
        self.sequence.iter().flat_map(|s| s.usages())
    }
}

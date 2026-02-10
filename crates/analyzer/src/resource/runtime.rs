//! Run-time defined Resources and Resource Groups (in analysis)

use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

use crate::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, OrderedStateTransitionCollector, State, Transition},
    resource::{Resource, ResourceCapacities, ResourceGroup},
};

/// Resource state transitions.
pub enum RtResourceStateTransition {
    Init(TimeUnixNanoSec),
    Operating(TimeUnixNanoSec, ResourceCapacities),
    Resizing(TimeUnixNanoSec),
    Finalizing(TimeUnixNanoSec),
    Exit(TimeUnixNanoSec),
}

/// Resource states.
#[derive(Debug)]
pub enum RtResourceState {
    Init(SpanUnixNanoSec),
    Operating(SpanUnixNanoSec, ResourceCapacities),
    Resizing(SpanUnixNanoSec),
    Finalizing(SpanUnixNanoSec),
}

impl Transition for RtResourceStateTransition {
    type Target = RtResourceState;
    fn timestamp(&self) -> TimeUnixNanoSec {
        *match self {
            RtResourceStateTransition::Init(ts) => ts,
            RtResourceStateTransition::Operating(ts, _) => ts,
            RtResourceStateTransition::Resizing(ts) => ts,
            RtResourceStateTransition::Finalizing(ts) => ts,
            RtResourceStateTransition::Exit(ts) => ts,
        }
    }

    fn try_into_state(self, end: TimeUnixNanoSec) -> AnalyzerResult<Self::Target> {
        Ok(match self {
            RtResourceStateTransition::Init(t) => {
                RtResourceState::Init(SpanUnixNanoSec::try_new(t, end)?)
            }
            RtResourceStateTransition::Operating(t, cap) => {
                RtResourceState::Operating(SpanUnixNanoSec::try_new(t, end)?, cap)
            }
            RtResourceStateTransition::Resizing(t) => {
                RtResourceState::Resizing(SpanUnixNanoSec::try_new(t, end)?)
            }
            RtResourceStateTransition::Finalizing(t) => {
                RtResourceState::Finalizing(SpanUnixNanoSec::try_new(t, end)?)
            }
            RtResourceStateTransition::Exit(_) => Err(AnalyzerError::FsmExitTransitionConversion)?,
        })
    }
}

impl State for RtResourceState {
    fn name(&self) -> &str {
        match self {
            RtResourceState::Init(_) => "init",
            RtResourceState::Operating(_, _) => "operating",
            RtResourceState::Resizing(_) => "resizing",
            RtResourceState::Finalizing(_) => "finalizing",
        }
    }

    fn span(&self) -> SpanUnixNanoSec {
        match self {
            RtResourceState::Init(span)
            | RtResourceState::Operating(span, _)
            | RtResourceState::Resizing(span)
            | RtResourceState::Finalizing(span) => *span,
        }
    }

    fn attributes(&self) -> impl Iterator<Item = &quent_attributes::Attribute> {
        std::iter::empty()
    }
}

pub struct RtResourceBuilder {
    id: Uuid,
    instance_name: Option<String>,
    type_name: Option<String>,
    parent_group_id: Option<Uuid>,
    transitions: OrderedStateTransitionCollector<RtResourceStateTransition>,
}

impl RtResourceBuilder {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::InvalidId(id))
        } else {
            Ok(Self {
                id,
                instance_name: None,
                type_name: None,
                parent_group_id: None,
                transitions: Default::default(),
            })
        }
    }
    pub fn set_type_name(&mut self, type_name: impl Into<String>) {
        self.type_name = Some(type_name.into());
    }
    pub fn set_instance_name(&mut self, instance_name: Option<String>) {
        self.instance_name = instance_name;
    }
    pub fn set_parent_group_id(&mut self, parent: Uuid) {
        self.parent_group_id = Some(parent);
    }
    pub fn push(&mut self, transition: RtResourceStateTransition) {
        self.transitions.push(transition);
    }
    pub fn try_build(self) -> AnalyzerResult<RtResource> {
        // Ensure >= 2 transitions
        let transitions: Vec<RtResourceStateTransition> = self.transitions.try_into()?;

        let mut sequence = Vec::with_capacity(transitions.len() - 1);
        let mut iter = transitions.into_iter();
        let mut current = iter.next().unwrap();
        for next in iter {
            let next_timestamp = next.timestamp();
            sequence.push(current.try_into_state(next_timestamp)?);
            current = next;
        }
        // TODO(johanpel): check transition logic.

        Ok(RtResource {
            id: self.id,
            instance_name: self.instance_name.ok_or_else(|| {
                AnalyzerError::IncompleteEntity(format!(
                    "resource {} must have an instance name",
                    self.id
                ))
            })?,
            type_name: self.type_name.ok_or_else(|| {
                AnalyzerError::IncompleteEntity(format!(
                    "resource {} must have a type name",
                    self.id
                ))
            })?,
            parent_group_id: self.parent_group_id.ok_or_else(|| {
                AnalyzerError::IncompleteEntity(format!(
                    "resource {} must have a parent resource group",
                    self.id
                ))
            })?,
            sequence,
        })
    }
}

/// A Resource.
#[derive(Debug)]
pub struct RtResource {
    /// The ID of this Resource.
    pub id: Uuid,
    /// The name of this Resource.
    pub instance_name: String,
    /// The unique type name of this Resource.
    pub type_name: String,
    /// The id of the parent Resource Group.
    pub parent_group_id: Uuid,
    /// The sequence of states that this resource went through.
    pub sequence: Vec<RtResourceState>,
}

impl Entity for RtResource {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        self.type_name.as_str()
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_str()
    }
}

impl Resource for RtResource {
    fn parent_group_id(&self) -> Uuid {
        self.parent_group_id
    }
}

impl Fsm for RtResource {
    type StateType = RtResourceState;
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

/// A Group of [`Resource`]s.
#[derive(Clone, Debug, Default)]
pub struct RtResourceGroup {
    /// The ID of this Resource Group.
    pub id: Uuid,
    /// The name of the type of Resource Group
    pub type_name: String,
    /// The name of the instance of this Resource Group.
    pub instance_name: String,
    /// The parent of this Resource Group.
    ///
    /// If this is None, it is considered the root of the global application's
    /// resource tree.
    pub parent_group_id: Option<Uuid>,
}

impl RtResourceGroup {
    pub fn try_new(
        id: Uuid,
        type_name: String,
        instance_name: String,
        parent_group_id: Option<Uuid>,
    ) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::InvalidId(id))
        } else {
            Ok(Self {
                id,
                type_name,
                instance_name,
                parent_group_id,
            })
        }
    }
}

impl Entity for RtResourceGroup {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        self.type_name.as_str()
    }

    fn instance_name(&self) -> &str {
        self.instance_name.as_str()
    }
}

impl ResourceGroup for RtResourceGroup {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.parent_group_id
    }
}

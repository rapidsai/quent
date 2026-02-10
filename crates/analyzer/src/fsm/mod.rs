//! FSM-related functionality

use quent_attributes::Attribute;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};

use crate::{AnalyzerResult, Entity, Span, error::AnalyzerError};

pub mod collection;
pub mod runtime;

/// Trait for types that represent an FSM state.
pub trait State {
    /// Return the unique name of the state.
    fn name(&self) -> &str;
    /// Return the span of time of this state.
    fn span(&self) -> SpanUnixNanoSec;
    /// Return an iterator over arbitrary key-value attributes associated with the state.
    fn attributes(&self) -> impl Iterator<Item = &Attribute>;
}

/// Trait for types that represent an FSM state transition.
pub trait Transition {
    type Target: State;

    /// Return the timestamp of the transition.
    fn timestamp(&self) -> TimeUnixNanoSec;
    /// Attempt to turn this transition into a [`Self::StateType`]
    fn try_into_state(self, end: TimeUnixNanoSec) -> AnalyzerResult<Self::Target>;
}

/// Trait for types that are an FSM.
pub trait Fsm: Entity {
    /// The type of the states of this FSM.
    ///
    /// Must implement [`State`].
    type StateType: State;

    /// Return the number of states.
    fn len(&self) -> usize;

    /// Return true if this FSM has no states, which means it is incomplete.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return a reference to the index-th state.
    ///
    /// If the index is out of bounds, this returns None.
    fn state(&self, index: usize) -> Option<&Self::StateType>;

    /// Return an iterator over all states.
    fn states(&self) -> impl ExactSizeIterator<Item = &Self::StateType>;

    /// Return the first state.
    fn first(&self) -> Option<&Self::StateType> {
        (!self.is_empty()).then(|| self.state(0).unwrap())
    }

    /// Return the last state.
    fn last(&self) -> Option<&Self::StateType> {
        (!self.is_empty()).then(|| self.state(self.len() - 1).unwrap())
    }
}

impl<U> Span for U
where
    U: Fsm + std::fmt::Debug,
{
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        if let Some(start) = self.first().map(|s| s.span().start())
            && let Some(end) = self.last().map(|s| s.span().end())
        {
            Ok(SpanUnixNanoSec::try_new(start, end)?)
        } else {
            Err(AnalyzerError::IncompleteEntity(format!(
                "fsm is incomplete: {self:?}"
            )))
        }
    }
}

/// Collects state transitions and inserts them in time order.
// The common case is for events to arrive in order, so a simple Vec
// will suffice to make the vast majority of ordered insertions fast.
//
// TODO(johanpel): since many FSMs have a known exact number of transitions,
// consider backing this with a SmallVec, requiring a const generic.
pub struct OrderedStateTransitionCollector<T>(Vec<T>)
where
    T: Transition;

impl<T> Default for OrderedStateTransitionCollector<T>
where
    T: Transition,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> OrderedStateTransitionCollector<T>
where
    T: Transition,
{
    pub fn push(&mut self, state: T) {
        if let Some(last) = self.0.last()
            && last.timestamp() <= state.timestamp()
        {
            self.0.push(state);
        } else {
            let pos = self
                .0
                .binary_search_by(|s| s.timestamp().cmp(&state.timestamp()))
                .unwrap_or_else(|i| i);
            self.0.insert(pos, state);
        }
    }
}

impl<T> Extend<T> for OrderedStateTransitionCollector<T>
where
    T: Transition,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for transition in iter {
            self.push(transition)
        }
    }
}

impl<T: Transition> TryFrom<OrderedStateTransitionCollector<T>> for Vec<T> {
    type Error = AnalyzerError;

    fn try_from(value: OrderedStateTransitionCollector<T>) -> AnalyzerResult<Self> {
        if value.0.len() >= 2 {
            Ok(value.0)
        } else {
            Err(AnalyzerError::IncompleteFsm(format!(
                "number of state transitions must be >= 2, is {}",
                value.0.len()
            )))
        }
    }
}

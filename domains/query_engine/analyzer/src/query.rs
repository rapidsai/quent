use quent_analyzer::{
    AnalyzerError, AnalyzerResult, Entity,
    fsm::{Fsm, OrderedStateTransitionCollector, State, Transition},
    resource::ResourceGroup,
};
use quent_events::Event;
use quent_query_engine_events::query::QueryEvent;
use quent_time::{TimeUnixNanoSec, span::SpanUnixNanoSec};
use uuid::Uuid;

pub enum QueryTransition {
    Init(TimeUnixNanoSec),
    Planning(TimeUnixNanoSec),
    Executing(TimeUnixNanoSec),
    Exit(TimeUnixNanoSec),
}

#[derive(Debug)]
pub enum QueryState {
    Init(SpanUnixNanoSec),
    Planning(SpanUnixNanoSec),
    Executing(SpanUnixNanoSec),
}

/// A query executed by an [`super::engine::Engine`].
#[derive(Debug)]
pub struct Query {
    /// The ID of this [`Query`].
    pub id: Uuid,
    /// The ID of the [`super::query_group::QueryGroup`] this query is part of.
    pub query_group_id: Option<Uuid>,
    /// A name for this [`Query`].
    pub instance_name: Option<String>,
    /// The sequence of states this [`Query`] went through.
    pub sequence: Vec<QueryState>,
}

pub struct QueryBuilder {
    pub id: Uuid,
    pub query_group_id: Option<Uuid>,
    pub instance_name: Option<String>,
    pub state_sequence: OrderedStateTransitionCollector<QueryTransition>,
}

impl QueryBuilder {
    pub fn try_new(id: Uuid) -> AnalyzerResult<Self> {
        if id.is_nil() {
            Err(AnalyzerError::Validation(
                "query id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                query_group_id: None,
                instance_name: None,
                state_sequence: Default::default(),
            })
        }
    }

    pub fn push(&mut self, event: Event<QueryEvent>) {
        match event.data {
            QueryEvent::Init(init) => {
                self.state_sequence
                    .push(QueryTransition::Init(event.timestamp));
                self.instance_name = Some(init.instance_name);
                self.query_group_id = Some(init.query_group_id);
            }
            QueryEvent::Planning => {
                self.state_sequence
                    .push(QueryTransition::Planning(event.timestamp));
            }
            QueryEvent::Executing => {
                self.state_sequence
                    .push(QueryTransition::Executing(event.timestamp));
            }
            QueryEvent::Exit => {
                self.state_sequence
                    .push(QueryTransition::Exit(event.timestamp));
            }
        }
    }

    pub fn try_build(self) -> AnalyzerResult<Query> {
        // TODO(johanpel): validate transitions

        let transitions: Vec<QueryTransition> = self.state_sequence.try_into()?;
        let mut sequence = Vec::with_capacity(transitions.len() - 1);
        let mut iter = transitions.into_iter();
        let mut current = iter.next().unwrap();
        for next in iter {
            let next_timestamp = next.timestamp();
            sequence.push(current.try_into_state(next_timestamp)?);
            current = next;
        }

        Ok(Query {
            id: self.id,
            query_group_id: self.query_group_id,
            instance_name: self.instance_name,
            sequence,
        })
    }
}

impl Entity for Query {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "query"
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or_default()
    }
}

impl Transition for QueryTransition {
    type Target = QueryState;

    fn timestamp(&self) -> TimeUnixNanoSec {
        *match self {
            QueryTransition::Init(t) => t,
            QueryTransition::Planning(t) => t,
            QueryTransition::Executing(t) => t,
            QueryTransition::Exit(t) => t,
        }
    }

    fn try_into_state(self, end: TimeUnixNanoSec) -> AnalyzerResult<Self::Target> {
        Ok(match self {
            QueryTransition::Init(t) => QueryState::Init(SpanUnixNanoSec::try_new(t, end)?),
            QueryTransition::Planning(t) => QueryState::Planning(SpanUnixNanoSec::try_new(t, end)?),
            QueryTransition::Executing(t) => {
                QueryState::Executing(SpanUnixNanoSec::try_new(t, end)?)
            }
            QueryTransition::Exit(_) => Err(AnalyzerError::FsmExitTransitionConversion)?,
        })
    }
}

impl State for QueryState {
    fn name(&self) -> &str {
        match self {
            QueryState::Init(_) => "init",
            QueryState::Planning(_) => "planning",
            QueryState::Executing(_) => "executing",
        }
    }

    fn span(&self) -> SpanUnixNanoSec {
        match self {
            QueryState::Init(span) | QueryState::Planning(span) | QueryState::Executing(span) => {
                *span
            }
        }
    }

    fn attributes(&self) -> impl Iterator<Item = &quent_attributes::Attribute> {
        std::iter::empty()
    }
}

impl Fsm for Query {
    type StateType = QueryState;
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

impl ResourceGroup for Query {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.query_group_id
    }
}

use py_rs::PY;
use quent_events::attributes::Attribute;
use quent_time::{Span, Timestamp};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{Entity, EntityRef, relation::Related, resource::Use};

#[derive(TS, PY, Clone, Debug, Deserialize, Serialize)]
pub struct State {
    pub name: String,
    pub uses: Vec<Use>,
    pub timestamp: Timestamp,
    pub attributes: Vec<Attribute>,
    pub relations: Vec<EntityRef>,
}

impl Related for State {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        self.relations.iter().cloned()
    }

    fn use_relations(&self) -> impl Iterator<Item = EntityRef> {
        self.uses.iter().map(|u| EntityRef::Resource(u.resource))
    }
}

#[derive(TS, PY, Clone, Default, Debug, Deserialize, Serialize)]
pub struct Fsm {
    pub id: Uuid,
    pub type_name: Option<String>,
    pub instance_name: Option<String>,
    pub state_sequence: Vec<State>,
    // TODO(johanpel): perhaps name these relations   pub relations: Vec<EntityRef>,
}

impl Fsm {
    pub fn state_span(&self, index: usize) -> Option<Span> {
        if index >= self.state_sequence.len() {
            // this is the exit state or some invalid index
            None
        } else {
            let start = self.state_sequence[index].timestamp;
            let end = self.state_sequence[index + 1].timestamp;
            Span::try_new(start, end).ok()
        }
    }
}

impl Entity for Fsm {
    fn new(id: Uuid) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Related for Fsm {
    fn relations(&self) -> impl Iterator<Item = EntityRef> {
        self.state_sequence
            .iter()
            .flat_map(|state| state.relations.iter())
            .cloned()
    }

    fn use_relations(&self) -> impl Iterator<Item = EntityRef> {
        self.state_sequence
            .iter()
            .flat_map(|state| state.use_relations())
    }
}

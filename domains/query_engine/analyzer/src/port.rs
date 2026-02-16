use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{Entity, resource::ResourceGroup};
use quent_attributes::{Attribute, Value};
use quent_events::Event;
use quent_query_engine_events::port::PortEvent;
use uuid::Uuid;

#[derive(Debug)]
pub struct PortStatistics {
    /// Custom statistics
    pub custom_statistics: HashMap<String, Option<Value>>,
}

/// A Port of an Operator in a Plan DAG.
#[derive(Debug)]
pub struct Port {
    /// The ID of this [`Port`]
    pub id: Uuid,
    /// The [`Operator`] to which this [`Port`] belongs.
    pub operator_id: Option<Uuid>,
    /// The name of this [`Port`].
    pub instance_name: Option<String>,
    /// The statistics of this [`Port`].
    ///
    /// These are attributes that are typically gathered after the work
    /// described by its associated [`Operator`] has completed.
    pub statistics: Option<PortStatistics>,
}

impl Port {
    pub fn try_new(id: Uuid) -> quent_analyzer::AnalyzerResult<Self> {
        if id.is_nil() {
            Err(quent_analyzer::AnalyzerError::Validation(
                "port id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                operator_id: None,
                instance_name: None,
                statistics: None,
            })
        }
    }

    pub fn push(&mut self, event: Event<PortEvent>) {
        match event.data {
            PortEvent::Declaration(declaration) => {
                self.operator_id = Some(declaration.operator_id);
                self.instance_name = Some(declaration.instance_name);
            }
            PortEvent::Statistics(statistics) => {
                self.statistics = Some(PortStatistics {
                    custom_statistics: statistics
                        .custom_attributes
                        .into_iter()
                        .map(|Attribute { key, value }| (key, value))
                        .collect(),
                });
            }
        }
    }
}

impl Entity for Port {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "port"
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or_default()
    }
}

impl ResourceGroup for Port {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.operator_id
    }
}

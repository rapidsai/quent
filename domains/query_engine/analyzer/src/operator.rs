use rustc_hash::FxHashMap as HashMap;

use quent_analyzer::{Entity, resource::ResourceGroup};
use quent_attributes::{Attribute, Value};
use quent_events::Event;
use quent_query_engine_events::operator::OperatorEvent;
use uuid::Uuid;

#[derive(Debug)]
pub struct OperatorStatistics {
    /// Custom statistics
    pub custom_statistics: HashMap<String, Option<Value>>,
}

/// An Operator in a Plan DAG.
#[derive(Debug)]
pub struct Operator {
    /// The ID of this [`Operator`].
    pub id: Uuid,
    /// The ID of the Plan this [`Operator`] belongs to.
    pub plan_id: Option<Uuid>,
    /// A list of [`Operator`] IDs in a parent plan (if any) from which this
    /// [`Operator`] was derived.
    pub parent_operator_ids: Vec<Uuid>,
    /// The name of this [`Operator`].
    pub instance_name: Option<String>,
    /// The name of this type of [`Operator`].
    pub operator_type_name: Option<String>,

    /// The custom attributes of this [`Operator`].
    pub custom_attributes: HashMap<String, Option<Value>>,
    /// The statistics of this [`Operator`].
    ///
    /// These are attributes that are typically gathered after the work
    /// described by an [`Operator`] has completed.
    pub statistics: Option<OperatorStatistics>,
}

impl Operator {
    pub fn try_new(id: Uuid) -> quent_analyzer::AnalyzerResult<Self> {
        if id.is_nil() {
            Err(quent_analyzer::AnalyzerError::Validation(
                "operator id cannot be nil".to_string(),
            ))
        } else {
            Ok(Self {
                id,
                plan_id: None,
                parent_operator_ids: Vec::new(),
                instance_name: None,
                operator_type_name: None,
                custom_attributes: HashMap::default(),
                statistics: None,
            })
        }
    }

    pub fn push(&mut self, event: Event<OperatorEvent>) {
        match event.data {
            OperatorEvent::Declaration(declaration) => {
                self.plan_id = Some(declaration.plan_id);
                self.parent_operator_ids = declaration.parent_operator_ids;
                self.instance_name = Some(declaration.instance_name);
                self.operator_type_name = Some(declaration.type_name);
                self.custom_attributes = declaration
                    .custom_attributes
                    .into_iter()
                    .map(|Attribute { key, value }| (key, value))
                    .collect();
            }
            OperatorEvent::Statistics(statistics) => {
                self.statistics = Some(OperatorStatistics {
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

impl Entity for Operator {
    fn id(&self) -> Uuid {
        self.id
    }
    fn type_name(&self) -> &str {
        "operator"
    }
    fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or_default()
    }
}

impl ResourceGroup for Operator {
    fn parent_group_id(&self) -> Option<Uuid> {
        self.plan_id
    }
}

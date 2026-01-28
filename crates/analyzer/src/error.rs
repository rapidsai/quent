use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum AnalyzerError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("invalid id: {0}")]
    InvalidId(Uuid),
    #[error("invalid id: {0}")]
    InvalidTypeName(String),
    #[error("invalid type name: {0}")]
    Logic(String),
    #[error("time error: {0}")]
    Time(#[from] quent_time::TimeError),
    #[error("entity error: {0}")]
    Entity(#[from] quent_entities::error::EntityError),
    #[error("value type error: {0}")]
    ValueType(String),
    #[error("broken implementation error: {0}")]
    BrokenImpl(String),
}

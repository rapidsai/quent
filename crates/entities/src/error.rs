use thiserror::Error;

/// Error type
#[derive(Error, Debug)]
pub enum EntityError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// Not all events needed to form a complete entity are present in the
    /// dataset.
    #[error("incomplete entity: {0}")]
    Incomplete(String),
    #[error("time error: {0}")]
    Time(#[from] quent_time::TimeError),
}

pub type Result<T> = std::result::Result<T, EntityError>;

use thiserror::Error;

/// Error type
#[derive(Error, Debug)]
pub enum EntityError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("incomplete fsm: {0}")]
    IncompleteFsm(String),
}

pub type Result<T> = std::result::Result<T, EntityError>;

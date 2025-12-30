use http::StatusCode;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("invalid id: {0}")]
    InvalidId(Uuid),
    #[error("logic error: {0}")]
    Logic(String),
}

impl From<Error> for StatusCode {
    fn from(value: Error) -> Self {
        match value {
            Error::Validation(_) | Error::Logic(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::InvalidId(_) => StatusCode::NOT_FOUND,
        }
    }
}

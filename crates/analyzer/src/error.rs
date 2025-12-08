use http::StatusCode;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("validation error: {0}")]
    Validation(String),
}

impl From<Error> for StatusCode {
    fn from(value: Error) -> Self {
        match value {
            Error::Validation(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

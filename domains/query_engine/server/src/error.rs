use axum::{http::StatusCode, response::IntoResponse};
use quent_analyzer::AnalyzerError;
use quent_exporter_types::ImporterError;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ServerError {
    #[error("analyzer error: {0}")]
    Analyzer(#[from] AnalyzerError),
    #[error("importer error: {0}")]
    Importer(#[from] ImporterError),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("cache error: {0}")]
    Cache(String),
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

pub(crate) type ServerResult<T> = std::result::Result<T, ServerError>;

impl From<ServerError> for StatusCode {
    fn from(value: ServerError) -> Self {
        match value {
            ServerError::Importer(_)
            | ServerError::Analyzer(_)
            | ServerError::Io(_)
            | ServerError::Cache(_)
            | ServerError::Join(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        let text = self.to_string();
        let status: StatusCode = self.into();
        (status, text).into_response()
    }
}

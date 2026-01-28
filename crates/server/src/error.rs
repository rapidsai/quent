use axum::{http::StatusCode, response::IntoResponse};
use quent_exporter::ImporterError;

#[derive(thiserror::Error, Debug)]
pub(crate) enum ServerError {
    #[error("analyzer error: {0}")]
    Analyzer(#[from] quent_analyzer::error::AnalyzerError),
    #[error("entity error: {0}")]
    Entity(#[from] quent_entities::error::EntityError),
    #[error("URL query parameters error: {0}")]
    UrlQueryParams(String),
    #[error("importer error: {0}")]
    Importer(#[from] ImporterError),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

pub(crate) type ServerResult<T> = std::result::Result<T, ServerError>;

impl From<ServerError> for StatusCode {
    fn from(value: ServerError) -> Self {
        match value {
            ServerError::UrlQueryParams(_) => StatusCode::BAD_REQUEST,
            ServerError::Importer(_)
            | ServerError::Analyzer(_)
            | ServerError::Io(_)
            | ServerError::Entity(_) => StatusCode::INTERNAL_SERVER_ERROR,
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

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use log::error;
use serde_json::json;
use std::io;
use thiserror::Error;

/// Errors returned by HTTP handlers. Each variant maps to a status code and a
/// JSON body of the form `{"error": "<message>"}`.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,
    #[error("no icon with id {0}")]
    IconNotFound(u64),
    #[error("no icon found for file '{0}'")]
    NoIconForFile(String),
    #[error("no icon found for folder '{0}'")]
    NoIconForFolder(String),
    #[error("internal server error")]
    Internal(#[from] io::Error),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound
            | Self::IconNotFound(_)
            | Self::NoIconForFile(_)
            | Self::NoIconForFolder(_) => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Self::Internal(source) = &self {
            // Log the real cause, but don't leak it to the client.
            error!("internal error: {source}");
        }

        (self.status(), Json(json!({ "error": self.to_string() }))).into_response()
    }
}

use crate::controllers::health::health;
use crate::controllers::icon::icon;
use crate::controllers::lookup::{file_icon, folder_icon};
use crate::errors::api::ApiError;
use axum::{Router, routing::get};

async fn not_found_fallback() -> ApiError {
    ApiError::NotFound
}

pub fn init_routes() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/icon/{id}", get(icon))
        .route("/file-icon", get(file_icon))
        .route("/folder-icon", get(folder_icon))
        .fallback(not_found_fallback)
}

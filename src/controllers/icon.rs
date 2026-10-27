use crate::errors::api::ApiError;
use axum::{extract::Path, http::header, response::IntoResponse};

pub async fn icon(Path(id): Path<u64>) -> Result<impl IntoResponse, ApiError> {
    let contents = iconator::get_icon_svg(id).ok_or(ApiError::IconNotFound(id))?;

    Ok((
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        contents,
    ))
}

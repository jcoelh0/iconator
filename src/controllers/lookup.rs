use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};
use crate::errors::api::ApiError;

#[derive(Debug, Deserialize)]
pub struct LookupQuery {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct LookupResponse {
    pub id: u64,
}

pub async fn file_icon(
    Query(LookupQuery { path }): Query<LookupQuery>,
) -> Result<Json<LookupResponse>, ApiError> {
    match iconator::get_icon_for_file(&path) {
        Some(id) => Ok(Json(LookupResponse { id })),
        None => Err(ApiError::NoIconForFile(path)),
    }
}

pub async fn folder_icon(
    Query(LookupQuery { path }): Query<LookupQuery>,
) -> Result<Json<LookupResponse>, ApiError> {
    match iconator::get_icon_for_folder(&path) {
        Some(id) => Ok(Json(LookupResponse { id })),
        None => Err(ApiError::NoIconForFolder(path)),
    }
}

#[cfg(test)]
mod test {
    use crate::routes::init_routes;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    fn server() -> TestServer {
        TestServer::new(init_routes())
    }

    #[tokio::test]
    async fn file_by_extension() {
        let res = server()
            .get("/file-icon")
            .add_query_param("path", "my-app/src/main.rs")
            .await;
        res.assert_status_ok();
        res.assert_json(&serde_json::json!({ "id": 525 }));
    }

    #[tokio::test]
    async fn folder_nested_path() {
        let res = server()
            .get("/folder-icon")
            .add_query_param("path", "my-app/src")
            .await;
        res.assert_status_ok();
        res.assert_json(&serde_json::json!({ "id": 1054 }));
    }

    #[tokio::test]
    async fn folder_trailing_slash() {
        let res = server()
            .get("/folder-icon")
            .add_query_param("path", "my-app/src/")
            .await;
        res.assert_json(&serde_json::json!({ "id": 1054 }));
    }

    #[tokio::test]
    async fn unknown_folder_is_404() {
        let res = server()
            .get("/folder-icon")
            .add_query_param("path", "my-app/definitely-not-a-known-folder")
            .await;
        res.assert_status(StatusCode::NOT_FOUND);
        res.assert_json(&serde_json::json!({
            "error": "no icon found for folder 'my-app/definitely-not-a-known-folder'"
        }));
    }

    #[tokio::test]
    async fn known_file_without_extension() {
        let res = server()
            .get("/file-icon")
            .add_query_param("path", "my-app/license")
            .await;
        res.assert_status_ok();
        res.assert_json(&serde_json::json!({ "id": 317 }));
    }

    #[tokio::test]
    async fn unknown_file_without_extension_is_404() {
        let res = server()
            .get("/file-icon")
            .add_query_param("path", "my-app/some-unknown-file")
            .await;
        res.assert_status(StatusCode::NOT_FOUND);
        res.assert_json(&serde_json::json!({
            "error": "no icon found for file 'my-app/some-unknown-file'"
        }));
    }

    #[tokio::test]
    async fn unknown_icon_id_is_404() {
        let res = server().get("/icon/999999").await;
        res.assert_status(StatusCode::NOT_FOUND);
        res.assert_json(&serde_json::json!({ "error": "no icon with id 999999" }));
    }

    #[tokio::test]
    async fn missing_path_is_400() {
        let res = server().get("/file-icon").await;
        res.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn icon_response_has_immutable_cache_header() {
        let res = server().get("/icon/1").await;
        res.assert_status_ok();
        res.assert_header("content-type", "image/svg+xml");
        res.assert_header("cache-control", "public, max-age=31536000, immutable");
    }

    #[tokio::test]
    async fn health_check_returns_json() {
        let res = server().get("/health").await;
        res.assert_status_ok();
        res.assert_json(&serde_json::json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn unmatched_route_returns_json_404() {
        let res = server().get("/non-existent-route").await;
        res.assert_status(StatusCode::NOT_FOUND);
        res.assert_json(&serde_json::json!({ "error": "not found" }));
    }
}

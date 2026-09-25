use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::json;
use utoipa::ToSchema;

use crate::state::AppState;
use super::SimpleErrorResponse;

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct PackageInfoResponse {
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
}

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct PackagesCacheResponse {
    pub total_packages: usize,
    pub total_size_bytes: u64,
    pub packages: Vec<PackageInfoResponse>,
}

#[utoipa::path(
    get,
    path = "/admin/packages",
    responses(
        (status = 200, description = "List of cached packages and storage size", body = PackagesCacheResponse),
        (status = 500, description = "Internal server error", body = SimpleErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn list_packages_handler(
    _state: State<AppState>,
) -> Result<Json<PackagesCacheResponse>, (StatusCode, Json<serde_json::Value>)> {
    let cache_info = crate::packages::get_cached_packages().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
    })?;

    Ok(Json(PackagesCacheResponse {
        total_packages: cache_info.total_packages,
        total_size_bytes: cache_info.total_size_bytes,
        packages: cache_info
            .packages
            .into_iter()
            .map(|p| PackageInfoResponse {
                name: p.name,
                version: p.version,
                size_bytes: p.size_bytes,
            })
            .collect(),
    }))
}

#[derive(serde::Deserialize, ToSchema)]
pub(crate) struct PreloadPackagesRequest {
    #[schema(example = json!(["@preview/cetz:0.3.1", "@preview/tablex:0.0.8"]))]
    pub packages: Vec<String>,
}

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct SimpleSuccessResponse {
    pub status: String,
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/admin/packages/preload",
    request_body = PreloadPackagesRequest,
    responses(
        (status = 200, description = "Packages preloaded successfully", body = SimpleSuccessResponse),
        (status = 500, description = "Internal server error", body = SimpleErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn preload_packages_handler(
    State(state): State<AppState>,
    Json(payload): Json<PreloadPackagesRequest>,
) -> Result<Json<SimpleSuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let packages_store = state.packages.clone();
    
    // Do it in a blocking task since downloading can block the thread
    let result = tokio::task::spawn_blocking(move || {
        crate::packages::preload_specific(packages_store, payload.packages)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))))?;

    match result {
        Ok(_) => Ok(Json(SimpleSuccessResponse {
            status: "ok".to_string(),
            message: "Packages preloaded successfully".to_string(),
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e})))),
    }
}

#[utoipa::path(
    post,
    path = "/admin/packages/sync-all",
    responses(
        (status = 202, description = "Sync task started", body = SimpleSuccessResponse),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn sync_all_packages_handler(
    State(state): State<AppState>,
) -> Result<Json<SimpleSuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let packages_store = state.packages.clone();
    tokio::spawn(async move {
        let _ = crate::packages::sync_missing_packages(packages_store).await;
    });

    Ok(Json(SimpleSuccessResponse {
        status: "accepted".to_string(),
        message: "Background sync task started".to_string(),
    }))
}

#[utoipa::path(
    delete,
    path = "/admin/packages/cache",
    responses(
        (status = 200, description = "Cache cleared successfully", body = SimpleSuccessResponse),
        (status = 500, description = "Internal server error", body = SimpleErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn clear_packages_cache_handler(
    _state: State<AppState>,
) -> Result<Json<SimpleSuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    crate::packages::clear_cache().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
    })?;

    Ok(Json(SimpleSuccessResponse {
        status: "ok".to_string(),
        message: "Package cache cleared successfully".to_string(),
    }))
}

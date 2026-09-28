use axum::{extract::State, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub(crate) struct AdminInfoResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub config: AdminConfigInfo,
    pub stats: AdminStatsInfo,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct AdminConfigInfo {
    pub port: u16,
    pub max_payload_size_mb: f64,
    pub compilation_timeout_secs: u64,
    pub max_concurrent_compilations: usize,
    pub auth_enabled: bool,
    pub admin_auth_enabled: bool,
    pub cors_enabled: bool,
    pub cache_all_packages: bool,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct AdminStatsInfo {
    pub fonts_loaded: usize,
}

#[utoipa::path(
    get,
    path = "/admin/info",
    responses(
        (status = 200, description = "Admin Information", body = AdminInfoResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn admin_info_handler(
    State(state): State<AppState>,
) -> Json<AdminInfoResponse> {
    let uptime_seconds = state.startup_time.elapsed().as_secs();

    let font_state = state.font_state.read().await;
    let fonts_loaded = font_state.fonts.len();
    drop(font_state);

    let auth_enabled = state.config.auth_token.is_some();
    let admin_auth_enabled = state.config.admin_token.is_some();

    let status = (if !auth_enabled
        && !admin_auth_enabled
        || state.config.cors_allowed_origins.as_deref() == Some("*")
    {
        "warning"
    } else {
        "ok"
    })
    .to_string();

    let max_payload_size_mb = state.config.max_payload_size as f64 / (1024.0 * 1024.0);

    let info = AdminInfoResponse {
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        config: AdminConfigInfo {
            port: state.config.port,
            max_payload_size_mb: (max_payload_size_mb * 100.0).round() / 100.0,
            compilation_timeout_secs: state.config.compilation_timeout_secs,
            max_concurrent_compilations: state.config.max_concurrent_compilations,
            auth_enabled,
            admin_auth_enabled,
            cors_enabled: state.config.cors_allowed_origins.is_some(),
            cache_all_packages: state.config.cache_all_packages,
        },
        stats: AdminStatsInfo {
            fonts_loaded,
        },
    };

    Json(info)
}

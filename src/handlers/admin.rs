use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub(crate) struct AdminInfoResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
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
    pub cache_all_133_typst_fonts: bool,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct AdminStatsInfo {
    pub fonts_loaded: usize,
    pub packages_cached: usize,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct UploadFontsResponse {
    pub fonts_added: usize,
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
pub(crate) async fn admin_info_handler(State(state): State<AppState>) -> Json<AdminInfoResponse> {
    let uptime_seconds = state.startup_time.elapsed().as_secs();

    let font_state = state.font_state.read().await;
    let fonts_loaded = font_state.fonts.len();
    drop(font_state);

    let auth_enabled = state.config.auth_token.is_some();
    let admin_auth_enabled = state.config.admin_token.is_some();

    let status = (if !auth_enabled && !admin_auth_enabled
        || state.config.cors_allowed_origins.as_deref() == Some("*")
    {
        "warning"
    } else {
        "ok"
    })
    .to_string();

    let mut warning = None;
    if status == "warning" {
        let mut reasons = Vec::new();
        if !admin_auth_enabled && !auth_enabled {
            reasons.push("Admin authentication is disabled.");
        }
        if state.config.cors_allowed_origins.as_deref() == Some("*") {
            reasons.push("CORS is allowed for all origins.");
        }
        warning = Some(reasons.join(" "));
    }

    let max_payload_size_mb = state.config.max_payload_size as f64 / (1024.0 * 1024.0);

    let packages_cached = crate::packages::get_cached_packages()
        .map(|info| info.total_packages)
        .unwrap_or(0);

    let info = AdminInfoResponse {
        status,
        warning,
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
            cache_all_133_typst_fonts: state.config.cache_all_mirror_fonts,
        },
        stats: AdminStatsInfo {
            fonts_loaded,
            packages_cached,
        },
    };

    Json(info)
}

#[utoipa::path(
    post,
    path = "/admin/fonts/sync",
    responses(
        (status = 202, description = "Sync started")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn sync_mirror_fonts_handler(State(state): State<AppState>) -> impl IntoResponse {
    let config_clone = state.config.clone();
    let font_state_clone = state.font_state.clone();

    tokio::spawn(async move {
        if let Err(e) =
            crate::font_cache::sync_mirror(config_clone.clone(), font_state_clone.clone()).await
        {
            tracing::error!("Background font sync task failed: {}", e);
        } else {
            tracing::info!("Background font sync task complete.");
        }
    });

    (StatusCode::ACCEPTED, "Sync started".to_string())
}

#[utoipa::path(
    post,
    path = "/admin/fonts",
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Fonts uploaded successfully", body = UploadFontsResponse),
        (status = 400, description = "Bad request")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn upload_fonts_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadFontsResponse>, (StatusCode, String)> {
    let font_dir = {
        let mut writable_dir = None;
        for path in &state.config.font_paths {
            if !path.exists() {
                if std::fs::create_dir_all(path).is_ok() {
                    writable_dir = Some(path.clone());
                    break;
                }
            } else {
                let test_file = path.join(".typst_api_write_test");
                if std::fs::write(&test_file, b"").is_ok() {
                    let _ = std::fs::remove_file(test_file);
                    writable_dir = Some(path.clone());
                    break;
                }
            }
        }
        writable_dir.unwrap_or_else(|| {
            state
                .config
                .font_paths
                .first()
                .cloned()
                .unwrap_or_else(|| std::path::PathBuf::from("./fonts"))
        })
    };

    if !font_dir.exists() {
        std::fs::create_dir_all(&font_dir).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create fonts directory: {}", e),
            )
        })?;
    }

    let mut fonts_added = 0;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Error reading multipart field: {}", e),
        )
    })? {
        if field.name() == Some("fonts") {
            let file_name = field.file_name().map(|s| s.to_string()).unwrap_or_else(|| {
                format!(
                    "uploaded_font_{}.ttf",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
                )
            });

            let ext = std::path::Path::new(&file_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if !["ttf", "otf", "ttc", "otc"].contains(&ext.as_str()) {
                continue;
            }

            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read file bytes: {}", e),
                )
            })?;

            let file_path = font_dir.join(&file_name);
            std::fs::write(&file_path, data).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to write file to disk: {}", e),
                )
            })?;

            fonts_added += 1;
        }
    }

    if fonts_added > 0 {
        tracing::info!(
            "Rebuilding FontState after uploading {} fonts...",
            fonts_added
        );
        let new_font_state =
            std::sync::Arc::new(crate::fonts::FontState::new(&state.config.font_paths));
        let mut guard = state.font_state.write().await;
        *guard = new_font_state;
        tracing::info!("FontState successfully rebuilt.");
    }

    Ok(Json(UploadFontsResponse { fonts_added }))
}

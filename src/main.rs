mod config;
mod fonts;
mod world;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, State},
    http::{HeaderMap, StatusCode, HeaderValue, Method, header},
    response::IntoResponse,
    routing::{get, post},
};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::CorsLayer;
use config::Config;
use fonts::FontState;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::timeout;
use typst::diag::SourceDiagnostic;
use typst::{World, WorldExt};
use typst_kit::downloader::SystemDownloader;
use typst_kit::packages::SystemPackages;
use world::ApiWorld;

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    font_state: Arc<tokio::sync::RwLock<Arc<FontState>>>,
    packages: Arc<SystemPackages>,
}

#[derive(serde::Deserialize)]
struct CompileSourceRequest {
    source: String,
    filename: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::load();
    tracing::info!("Starting server on port {}", config.port);
    tracing::info!("Font paths: {:?}", config.font_paths);

    let font_state = Arc::new(tokio::sync::RwLock::new(Arc::new(FontState::new(&config.font_paths))));

    let downloader = SystemDownloader::new("typst-api");
    let packages = Arc::new(SystemPackages::new(downloader));

    let state = AppState {
        config: Arc::new(config.clone()),
        font_state,
        packages,
    };

    let cors_layer = if let Some(cors_origins) = &config.cors_allowed_origins {
        if cors_origins == "*" {
            CorsLayer::permissive()
        } else {
            let allowed_patterns: Vec<String> = cors_origins
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            
            let allow_origin = tower_http::cors::AllowOrigin::predicate(
                move |origin: &HeaderValue, _| {
                    if let Ok(origin_str) = origin.to_str() {
                        allowed_patterns.iter().any(|pattern| {
                            origin_str == pattern || origin_str.starts_with(&format!("{}:", pattern))
                        })
                    } else {
                        false
                    }
                },
            );

            CorsLayer::new()
                .allow_origin(allow_origin)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        }
    } else {
        CorsLayer::new() // Default restrictive layer
    };

    let compile_router = Router::new()
        .route("/", post(compile_handler))
        .route("/source", post(compile_source_handler))
        .layer(ConcurrencyLimitLayer::new(config.max_concurrent_compilations));

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/compile", compile_router)
        .route("/fonts", get(list_fonts_handler))
        .route("/fonts/refresh", post(refresh_fonts_handler))
        .layer(cors_layer)
        .layer(DefaultBodyLimit::max(config.max_payload_size))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn compile_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if let Some(token) = &state.config.auth_token {
        let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
        let expected = format!("Bearer {}", token);
        if auth_header != Some(&expected) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            ));
        }
    }

    let mut files_data = HashMap::new();
    let mut main_file = None;
    let mut ephemeral_fonts = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().unwrap_or("").to_string();

        if let Ok(data) = field.bytes().await {
            let data = data.to_vec();
            
            let is_font = file_name.ends_with(".ttf") || file_name.ends_with(".otf") || file_name.ends_with(".ttc");

            if is_font {
                let bytes = typst::foundations::Bytes::new(data);
                for font in typst::text::Font::iter(bytes) {
                    ephemeral_fonts.push(font);
                }
            } else if name == "main" {
                let fname = if file_name.is_empty() {
                    "main.typ".to_string()
                } else {
                    file_name.clone()
                };
                main_file = Some(fname.clone());
                files_data.insert(fname, data);
            } else {
                let target_path = if !name.is_empty() { name } else { file_name };
                if !target_path.is_empty() {
                    files_data.insert(target_path, data);
                }
            }
        }
    }

    if main_file.is_none() && files_data.contains_key("main.typ") {
        main_file = Some("main.typ".to_string());
    }

    let main_file = match main_file {
        Some(m) => m,
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "No main file specified or main.typ found"})),
            ));
        }
    };

    let global_font_state = state.font_state.read().await.clone();
    let world = match ApiWorld::new(
        global_font_state,
        ephemeral_fonts,
        state.packages.clone(),
        files_data,
        main_file,
    ) {
        Ok(w) => Arc::new(w),
        Err(e) => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": e})))),
    };

    perform_compilation(world, state.config.compilation_timeout_secs).await
}

async fn perform_compilation(
    world: Arc<ApiWorld>,
    timeout_secs: u64,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let timeout_duration = std::time::Duration::from_secs(timeout_secs);

    let world_clone = world.clone();
    let compile_result = timeout(
        timeout_duration,
        tokio::task::spawn_blocking(move || typst::compile(&*world_clone)),
    )
    .await;

    match compile_result {
        Ok(Ok(warned)) => match warned.output {
            Ok(document) => {
                let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default());
                Ok((
                    StatusCode::OK,
                    [("Content-Type", "application/pdf")],
                    pdf.unwrap_or_default(),
                ))
            }
            Err(errors) => {
                let formatted_errors = format_errors(&world, &errors);
                Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "errors": formatted_errors })),
                ))
            }
        },
        Ok(Err(_)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Compilation task panicked"})),
        )),
        Err(_) => Err((
            StatusCode::REQUEST_TIMEOUT,
            Json(json!({"error": "Compilation timed out"})),
        )),
    }
}

async fn compile_source_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CompileSourceRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if let Some(token) = &state.config.auth_token {
        let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
        let expected = format!("Bearer {}", token);
        if auth_header != Some(&expected) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            ));
        }
    }

    let main_file = payload.filename.unwrap_or_else(|| "main.typ".to_string());
    let mut files_data = HashMap::new();
    files_data.insert(main_file.clone(), payload.source.into_bytes());

    let global_font_state = state.font_state.read().await.clone();
    let world = match ApiWorld::new(
        global_font_state,
        vec![],
        state.packages.clone(),
        files_data,
        main_file,
    ) {
        Ok(w) => Arc::new(w),
        Err(e) => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": e})))),
    };

    perform_compilation(world, state.config.compilation_timeout_secs).await
}

fn format_errors(world: &ApiWorld, errors: &[SourceDiagnostic]) -> Vec<serde_json::Value> {
    errors
        .iter()
        .map(|e| {
            let mut info = json!({
                "message": e.message,
                "severity": format!("{:?}", e.severity),
            });

            let map = info.as_object_mut().unwrap();

            if let Some(id) = e.span.id() {
                map.insert("file".to_string(), json!(format!("{:?}", id)));
                if let Ok(source) = world.source(id) {
                    if let Some(range) = world.range(e.span) {
                        if let Some((line, col)) = source.lines().byte_to_line_column(range.start) {
                            map.insert("line".to_string(), json!(line + 1));
                            map.insert("column".to_string(), json!(col + 1));
                        }
                    }
                }
            }

            if !e.hints.is_empty() {
                let hints: Vec<String> = e.hints.iter().map(|h| h.v.to_string()).collect();
                map.insert("hints".to_string(), json!(hints));
            }

            info
        })
        .collect()
}

async fn list_fonts_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let font_state = state.font_state.read().await;
    let mut fonts = Vec::new();
    for font in &font_state.fonts {
        let info = font.info();
        fonts.push(json!({
            "family": info.family,
            "style": format!("{:?}", info.variant.style),
            "weight": info.variant.weight.to_number(),
            "stretch": format!("{:?}", info.variant.stretch),
        }));
    }
    Json(json!({ "fonts": fonts }))
}

async fn refresh_fonts_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(token) = &state.config.auth_token {
        let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
        let expected = format!("Bearer {}", token);
        if auth_header != Some(&expected) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            ));
        }
    }

    let mut font_state = state.font_state.write().await;
    *font_state = Arc::new(FontState::new(&state.config.font_paths));
    Ok(Json(json!({ "status": "ok", "message": "Fonts reloaded successfully" })))
}

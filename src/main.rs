mod config;
mod fonts;
mod world;

use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use config::Config;
use fonts::FontState;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::timeout;
use typst::diag::SourceDiagnostic;
use world::ApiWorld;

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    font_state: Arc<FontState>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::load();
    tracing::info!("Starting server on port {}", config.port);
    tracing::info!("Font paths: {:?}", config.font_paths);

    let font_state = Arc::new(FontState::new(&config.font_paths));

    let state = AppState {
        config: Arc::new(config.clone()),
        font_state,
    };

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/compile", post(compile_handler))
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
            return Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))));
        }
    }

    let mut files_data = HashMap::new();
    let mut main_file = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().unwrap_or("").to_string();
        
        if let Ok(data) = field.bytes().await {
            let data = data.to_vec();
            if name == "main" {
                let fname = if file_name.is_empty() { "main.typ".to_string() } else { file_name.clone() };
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
            return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "No main file specified or main.typ found"}))));
        }
    };

    let world = match ApiWorld::new(state.font_state.clone(), files_data, main_file) {
        Ok(w) => w,
        Err(e) => return Err((StatusCode::BAD_REQUEST, Json(json!({"error": e})))),
    };

    let timeout_duration = std::time::Duration::from_secs(state.config.compilation_timeout_secs);

    let compile_result = timeout(timeout_duration, tokio::task::spawn_blocking(move || {
        typst::compile(&world)
    })).await;

    match compile_result {
        Ok(Ok(warned)) => {
            match warned.output {
                Ok(document) => {
                    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default());
                    Ok((
                        StatusCode::OK,
                        [("Content-Type", "application/pdf")],
                        pdf.unwrap_or_default(),
                    ))
                }
                Err(errors) => {
                    let formatted_errors = format_errors(&errors);
                    Err((StatusCode::BAD_REQUEST, Json(json!({ "errors": formatted_errors }))))
                }
            }
        }
        Ok(Err(_)) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Compilation task panicked"}))))
        }
        Err(_) => {
            Err((StatusCode::REQUEST_TIMEOUT, Json(json!({"error": "Compilation timed out"}))))
        }
    }
}

fn format_errors(errors: &[SourceDiagnostic]) -> Vec<serde_json::Value> {
    errors.iter().map(|e| {
        json!({
            "message": e.message,
            "severity": format!("{:?}", e.severity),
        })
    }).collect()
}

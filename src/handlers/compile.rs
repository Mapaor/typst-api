use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::timeout;
use typst::diag::SourceDiagnostic;
use typst::{World, WorldExt};

use crate::state::AppState;
use crate::world::ApiWorld;
use utoipa::ToSchema;

#[derive(serde::Deserialize, ToSchema)]
pub(crate) struct CompileSourceRequest {
    #[schema(example = "= Hello Typst!\nThis is a simple document.")]
    pub(crate) source: String,
    #[schema(example = "main.typ")]
    pub(crate) filename: Option<String>,
}

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct ErrorInfo {
    pub message: String,
    pub severity: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub hints: Option<Vec<String>>,
}

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct ErrorResponse {
    pub errors: Vec<ErrorInfo>,
}

use super::SimpleErrorResponse;

#[derive(ToSchema)]
#[allow(dead_code)]
pub(crate) struct CompileRequest {
    /// The main typst file. Field can be named `main` or anything if the filename ends with `.typ`
    #[schema(value_type = String, format = Binary)]
    pub main: String,
    /// Any additional files (images, typst modules, fonts)
    #[schema(value_type = Option<Vec<String>>, format = Binary)]
    pub _other_files: Option<Vec<String>>,
}

#[utoipa::path(
    post,
    path = "/compile",
    request_body(content_type = "multipart/form-data", content = CompileRequest),
    responses(
        (status = 200, description = "Successfully compiled PDF", body = [u8], content_type = "application/pdf"),
        (status = 400, description = "Compilation error or bad request", body = ErrorResponse),
        (status = 408, description = "Compilation timed out", body = SimpleErrorResponse),
        (status = 500, description = "Internal server error", body = SimpleErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn compile_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
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

#[utoipa::path(
    post,
    path = "/compile/source",
    request_body = CompileSourceRequest,
    responses(
        (status = 200, description = "Successfully compiled PDF", body = [u8], content_type = "application/pdf"),
        (status = 400, description = "Compilation error or bad request", body = ErrorResponse),
        (status = 408, description = "Compilation timed out", body = SimpleErrorResponse),
        (status = 500, description = "Internal server error", body = SimpleErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn compile_source_handler(
    State(state): State<AppState>,
    Json(payload): Json<CompileSourceRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
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
                if let Ok(source) = world.source(id)
                    && let Some(range) = world.range(e.span)
                    && let Some((line, col)) = source.lines().byte_to_line_column(range.start)
                {
                    map.insert("line".to_string(), json!(line + 1));
                    map.insert("column".to_string(), json!(col + 1));
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

#[cfg(test)]
mod tests {
    use axum::{
        http::{Request, StatusCode},
        body::Body,
    };
    use tower::ServiceExt;
    use crate::create_app;
    use crate::state::test_helpers::create_test_state;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn test_compile_source_success() {
        let state = create_test_state(None);
        let app = create_app(state);
        
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Title\nHello world!"}"#))
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers().get("Content-Type").unwrap(), "application/pdf");
        
        let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        assert!(body_bytes.starts_with(b"%PDF-"));
    }

    #[tokio::test]
    async fn test_compile_source_error() {
        let state = create_test_state(None);
        let app = create_app(state);
        
        // Intentional syntax error in typst code (unclosed bracket)
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Title\n#let x = (1, 2"}"#))
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        
        let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        
        assert!(body_json.get("errors").is_some());
        let errors = body_json["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
    }
}

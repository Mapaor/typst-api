use axum::{
    extract::{Multipart, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::sync::Arc;
use tokio::time::timeout;
use typst::diag::SourceDiagnostic;
use typst::{World, WorldExt};

use crate::state::AppState;
use crate::world::ApiWorld;
use utoipa::{IntoParams, ToSchema};
use zip::write::SimpleFileOptions;

#[derive(serde::Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct CompileOptions {
    /// Output format. Defaults to PDF.
    #[param(default = "pdf")]
    pub(crate) format: Option<String>,
    /// 1-based pages and ranges to export, for example `2,3,7-9,11-`.
    pub(crate) pages: Option<String>,
}

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
    params(CompileOptions),
    responses(
        (status = 200, description = "Compiled document. The response is PDF, a single SVG page, or a ZIP archive of multiple SVG pages.", content(
            ([u8] = "application/pdf"),
            ([u8] = "image/svg+xml"),
            ([u8] = "application/zip")
        )),
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
    Query(options): Query<CompileOptions>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let options = parse_compile_options(options)?;
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

    perform_compilation(world, state.config.compilation_timeout_secs, options).await
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Pdf,
    Svg,
}

struct ParsedCompileOptions {
    format: OutputFormat,
    pages: Option<Vec<(usize, Option<usize>)>>,
}

fn parse_compile_options(
    options: CompileOptions,
) -> Result<ParsedCompileOptions, (StatusCode, Json<serde_json::Value>)> {
    let format = match options.format.as_deref().unwrap_or("pdf") {
        "pdf" => OutputFormat::Pdf,
        "svg" => OutputFormat::Svg,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Unsupported output format '{other}'. Use 'pdf' or 'svg'.")})),
            ));
        }
    };

    if options.pages.is_some() && format != OutputFormat::Svg {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "The pages parameter is only supported with format=svg"})),
        ));
    }

    let pages = options
        .pages
        .as_deref()
        .map(parse_page_selection)
        .transpose()
        .map_err(|error| (StatusCode::BAD_REQUEST, Json(json!({"error": error}))))?;

    Ok(ParsedCompileOptions { format, pages })
}

fn parse_page_selection(value: &str) -> Result<Vec<(usize, Option<usize>)>, String> {
    let mut pages = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err("Page selection contains an empty item".to_string());
        }

        if let Some((start, end)) = part.split_once('-') {
            let start = if start.is_empty() {
                1
            } else {
                parse_page_number(start)?
            };
            let end = if end.is_empty() {
                None
            } else {
                Some(parse_page_number(end)?)
            };
            if let Some(end) = end {
                if start > end {
                    return Err(format!("Invalid page range '{part}': start exceeds end"));
                }
            }
            pages.push((start, end));
        } else {
            let page = parse_page_number(part)?;
            pages.push((page, Some(page)));
        }
    }

    if pages.is_empty() {
        return Err("Page selection must not be empty".to_string());
    }
    Ok(pages)
}

fn parse_page_number(value: &str) -> Result<usize, String> {
    let page = value
        .parse::<usize>()
        .map_err(|_| format!("Invalid page number '{value}'"))?;
    if page == 0 {
        return Err("Page numbers start at 1".to_string());
    }
    Ok(page)
}

async fn perform_compilation(
    world: Arc<ApiWorld>,
    timeout_secs: u64,
    options: ParsedCompileOptions,
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
            Ok(document) => match options.format {
                OutputFormat::Pdf => {
                    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default());
                    Ok((
                        StatusCode::OK,
                        [("Content-Type", "application/pdf")],
                        pdf.unwrap_or_default(),
                    ))
                }
                OutputFormat::Svg => {
                    let selected_pages = match select_pages(options.pages.as_deref(), document.pages().len()) {
                        Ok(pages) => pages,
                        Err(error) => {
                            return Err((StatusCode::BAD_REQUEST, Json(json!({"error": error}))));
                        }
                    };
                    let svgs: Vec<(usize, String)> = selected_pages
                        .into_iter()
                        .map(|page| (page + 1, typst_svg::svg(&document.pages()[page], &typst_svg::SvgOptions::default())))
                        .collect();
                    if svgs.len() == 1 {
                        Ok((
                            StatusCode::OK,
                            [("Content-Type", "image/svg+xml")],
                            svgs.into_iter().next().unwrap().1.into_bytes(),
                        ))
                    } else {
                        match zip_svgs(svgs) {
                            Ok(bytes) => Ok((
                                StatusCode::OK,
                                [("Content-Type", "application/zip")],
                                bytes,
                            )),
                            Err(error) => Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({"error": format!("Failed to package SVG pages: {error}")})),
                            )),
                        }
                    }
                }
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
    params(CompileOptions),
    responses(
        (status = 200, description = "Compiled document. The response is PDF, a single SVG page, or a ZIP archive of multiple SVG pages.", content(
            ([u8] = "application/pdf"),
            ([u8] = "image/svg+xml"),
            ([u8] = "application/zip")
        )),
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
    Query(options): Query<CompileOptions>,
    Json(payload): Json<CompileSourceRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let options = parse_compile_options(options)?;
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

    perform_compilation(world, state.config.compilation_timeout_secs, options).await
}

fn select_pages(
    requested: Option<&[(usize, Option<usize>)]>,
    page_count: usize,
) -> Result<Vec<usize>, String> {
    if page_count == 0 {
        return Err("The compiled document contains no pages".to_string());
    }

    let Some(requested) = requested else {
        return Ok((0..page_count).collect());
    };

    let mut selected = vec![false; page_count];
    for &(start, end) in requested {
        let end = end.unwrap_or(page_count);
        if start > page_count {
            return Err(format!("Page {start} is out of range; document has {page_count} pages"));
        }
        if end > page_count {
            return Err(format!("Page {end} is out of range; document has {page_count} pages"));
        }
        for page in start..=end {
            selected[page - 1] = true;
        }
    }
    Ok(selected
        .into_iter()
        .enumerate()
        .filter_map(|(index, included)| included.then_some(index))
        .collect())
}

fn zip_svgs(svgs: Vec<(usize, String)>) -> Result<Vec<u8>, String> {
    let cursor = Cursor::new(Vec::new());
    let mut archive = zip::ZipWriter::new(cursor);
    for (page_number, svg) in svgs {
        archive
            .start_file(format!("page-{page_number}.svg"), SimpleFileOptions::default())
            .map_err(|error| error.to_string())?;
        archive
            .write_all(svg.as_bytes())
            .map_err(|error| error.to_string())?;
    }
    archive
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|error| error.to_string())
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
    use super::{parse_page_selection, select_pages};
    use std::io::Cursor;
    use axum::{
        http::{Request, StatusCode},
        body::Body,
    };
    use tower::ServiceExt;
    use crate::create_app;
    use crate::state::test_helpers::create_test_state;
    use axum::body::to_bytes;

    #[test]
    fn page_selection_parses_ranges_and_deduplicates_in_document_order() {
        let requested = parse_page_selection("2,3,7-9,11-").unwrap();
        assert_eq!(select_pages(Some(&requested), 12).unwrap(), vec![1, 2, 6, 7, 8, 10, 11]);
        assert_eq!(select_pages(Some(&requested), 10).unwrap_err(), "Page 11 is out of range; document has 10 pages");
    }

    #[test]
    fn page_selection_rejects_invalid_values() {
        assert!(parse_page_selection("0").is_err());
        assert!(parse_page_selection("1,,2").is_err());
        assert!(parse_page_selection("5-3").is_err());
        assert!(parse_page_selection("").is_err());
    }

    #[tokio::test]
    async fn test_compile_source_svg_single_page() {
        let state = create_test_state(None);
        let app = create_app(state);
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source?format=svg")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source":"= SVG Test\\nHello world!"}"#))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers().get("Content-Type").unwrap(), "image/svg+xml");
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        assert!(std::str::from_utf8(&body).unwrap().contains("<svg"));
    }

    #[tokio::test]
    async fn test_compile_source_svg_multiple_pages_returns_zip() {
        let state = create_test_state(None);
        let app = create_app(state);
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source?format=svg")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source":"Page one\\n#pagebreak()\\nPage two"}"#))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers().get("Content-Type").unwrap(), "application/zip");
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(body)).unwrap();
        assert_eq!(archive.len(), 2);
        assert!(archive.by_name("page-1.svg").is_ok());
        assert!(archive.by_name("page-2.svg").is_ok());
    }

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

    #[tokio::test]
    async fn test_compile_multipart_success() {
        let state = create_test_state(None);
        let app = create_app(state);
        
        let boundary = "------------------------14737809831466499882746641449";
        let body = format!(
            "--{0}\r\n\
            Content-Disposition: form-data; name=\"main\"; filename=\"main.typ\"\r\n\
            \r\n\
            = Hello World\n\
            #include \"other.typ\"\r\n\
            --{0}\r\n\
            Content-Disposition: form-data; name=\"other.typ\"; filename=\"other.typ\"\r\n\
            \r\n\
            This is from the other file.\r\n\
            --{0}--\r\n",
            boundary
        );
        
        let req = Request::builder()
            .method("POST")
            .uri("/compile")
            .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
            .body(Body::from(body))
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers().get("Content-Type").unwrap(), "application/pdf");
        
        let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        assert!(body_bytes.starts_with(b"%PDF-"));
    }
}

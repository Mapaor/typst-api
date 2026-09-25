use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::json;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::fonts::FontState;
use crate::state::AppState;

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct FontInfo {
    pub family: String,
    pub style: String,
    pub weight: u16,
    pub stretch: String,
}

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct FontsResponse {
    pub fonts: Vec<FontInfo>,
}

#[utoipa::path(
    get,
    path = "/fonts",
    responses(
        (status = 200, description = "List of available system and configured fonts", body = FontsResponse)
    )
)]
pub(crate) async fn list_fonts_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
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

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct RefreshFontsResponse {
    pub status: String,
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/admin/fonts/refresh",
    responses(
        (status = 200, description = "Fonts reloaded successfully", body = RefreshFontsResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub(crate) async fn refresh_fonts_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {

    let mut font_state = state.font_state.write().await;
    *font_state = Arc::new(FontState::new(&state.config.font_paths));
    Ok(Json(json!({ "status": "ok", "message": "Fonts reloaded successfully" })))
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
    async fn test_list_fonts() {
        let state = create_test_state(None);
        let app = create_app(state);
        
        let req = Request::builder()
            .method("GET")
            .uri("/fonts")
            .body(Body::empty())
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        
        let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(body_json.get("fonts").is_some());
    }
}

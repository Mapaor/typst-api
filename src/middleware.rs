use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::StatusCode,
    Json,
};
use serde_json::json;
use crate::AppState;

pub async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if let Some(token) = &state.config.auth_token {
        let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
        let expected = format!("Bearer {}", token);
        if auth_header != Some(&expected) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            ));
        }
    }
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use axum::{
        http::{Request, StatusCode, header},
        body::Body,
    };
    use tower::ServiceExt;
    use crate::{AppState, config::Config, fonts::FontState, create_app};
    use std::sync::Arc;
    use typst_kit::downloader::SystemDownloader;
    use typst_kit::packages::SystemPackages;
    
    fn create_test_state(token: Option<String>) -> AppState {
        let config = Config {
            port: 8080,
            font_paths: vec![],
            max_payload_size: 1024,
            compilation_timeout_secs: 10,
            auth_token: token,
            cors_allowed_origins: None,
            max_concurrent_compilations: 10,
        };
        let font_state = Arc::new(tokio::sync::RwLock::new(Arc::new(FontState::new(&config.font_paths))));
        let downloader = SystemDownloader::new("typst-api-test");
        let packages = Arc::new(SystemPackages::new(downloader));
        AppState {
            config: Arc::new(config),
            font_state,
            packages,
        }
    }
    
    #[tokio::test]
    async fn test_auth_disabled() {
        let state = create_test_state(None);
        let app = create_app(state);
        
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#))
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_missing_header() {
        let state = create_test_state(Some("secret".to_string()));
        let app = create_app(state);
        
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#))
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_invalid_token() {
        let state = create_test_state(Some("secret".to_string()));
        let app = create_app(state);
        
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header(header::AUTHORIZATION, "Bearer wrong")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#))
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_correct_token() {
        let state = create_test_state(Some("secret".to_string()));
        let app = create_app(state);
        
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header(header::AUTHORIZATION, "Bearer secret")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#))
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_public_routes() {
        let state = create_test_state(Some("secret".to_string()));
        let app = create_app(state);
        
        let req = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();
            
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let req = Request::builder()
            .method("GET")
            .uri("/fonts")
            .body(Body::empty())
            .unwrap();
            
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}

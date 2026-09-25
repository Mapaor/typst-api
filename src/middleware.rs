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
    if state.config.auth_token.is_some() || state.config.admin_token.is_some() {
        let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
        let mut allowed = false;

        if let Some(token) = &state.config.auth_token {
            let expected = format!("Bearer {}", token);
            if auth_header == Some(&expected) {
                allowed = true;
            }
        }
        
        if !allowed && let Some(token) = &state.config.admin_token {
            let expected = format!("Bearer {}", token);
            if auth_header == Some(&expected) {
                allowed = true;
            }
        }

        if !allowed {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            ));
        }
    }
    Ok(next.run(req).await)
}

pub async fn admin_auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if let Some(token) = &state.config.admin_token {
        let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
        let expected = format!("Bearer {}", token);
        if auth_header != Some(&expected) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized admin access"})),
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
    use crate::create_app;
    use crate::state::test_helpers::create_test_state;
    
    #[tokio::test]
    async fn test_auth_disabled() {
        let mut config = crate::config::Config::load();
        config.auth_token = None;
        config.admin_token = None;
        let state = create_test_state(None); // test helper uses this, need to ensure admin is None
        let mut state = state;
        state.config = std::sync::Arc::new(config);
        let app = create_app(state);
        
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#))
            .unwrap();
            
        let res = app.clone().oneshot(req).await.unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
        
        let req2 = Request::builder()
            .method("POST")
            .uri("/admin/fonts/refresh")
            .body(Body::empty())
            .unwrap();
        let res2 = app.oneshot(req2).await.unwrap();
        assert_ne!(res2.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_admin_only() {
        let mut config = crate::config::Config::load();
        config.auth_token = None;
        config.admin_token = Some("admin_secret".to_string());
        let mut state = create_test_state(None);
        state.config = std::sync::Arc::new(config);
        let app = create_app(state);
        
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header(header::AUTHORIZATION, "Bearer admin_secret")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#))
            .unwrap();
            
        let res = app.clone().oneshot(req).await.unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
        
        // Check admin token protects refresh endpoint
        let req2 = Request::builder()
            .method("POST")
            .uri("/admin/fonts/refresh")
            .body(Body::empty())
            .unwrap();
        let res2 = app.clone().oneshot(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::UNAUTHORIZED);
        
        let req3 = Request::builder()
            .method("POST")
            .uri("/admin/fonts/refresh")
            .header(header::AUTHORIZATION, "Bearer admin_secret")
            .body(Body::empty())
            .unwrap();
        let res3 = app.oneshot(req3).await.unwrap();
        assert_ne!(res3.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_both_tokens() {
        let mut config = crate::config::Config::load();
        config.auth_token = Some("user_secret".to_string());
        config.admin_token = Some("admin_secret".to_string());
        let mut state = create_test_state(None);
        state.config = std::sync::Arc::new(config);
        let app = create_app(state);
        
        // user can compile
        let req = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header(header::AUTHORIZATION, "Bearer user_secret")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
        
        // admin can compile
        let req2 = Request::builder()
            .method("POST")
            .uri("/compile/source")
            .header(header::AUTHORIZATION, "Bearer admin_secret")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#))
            .unwrap();
        let res2 = app.clone().oneshot(req2).await.unwrap();
        assert_ne!(res2.status(), StatusCode::UNAUTHORIZED);
        
        // user cannot refresh fonts
        let req3 = Request::builder()
            .method("POST")
            .uri("/admin/fonts/refresh")
            .header(header::AUTHORIZATION, "Bearer user_secret")
            .body(Body::empty())
            .unwrap();
        let res3 = app.clone().oneshot(req3).await.unwrap();
        assert_eq!(res3.status(), StatusCode::UNAUTHORIZED);
    }
}

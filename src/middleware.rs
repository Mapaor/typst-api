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
    if let Some(auth_token) = &state.config.auth_token {
        let auth_header = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
        let expected_auth = format!("Bearer {}", auth_token);

        let mut allowed = auth_header == Some(&expected_auth);

        if !allowed
            && let Some(admin_token) = &state.config.admin_token 
        {
            let expected_admin = format!("Bearer {}", admin_token);
            if auth_header == Some(&expected_admin) {
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
    
    fn req_compile(token: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method("POST").uri("/compile").header("Content-Type", "multipart/form-data; boundary=fake");
        if let Some(t) = token { b = b.header(header::AUTHORIZATION, format!("Bearer {}", t)); }
        b.body(Body::empty()).unwrap()
    }
    
    fn req_compile_source(token: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method("POST").uri("/compile/source").header("Content-Type", "application/json");
        if let Some(t) = token { b = b.header(header::AUTHORIZATION, format!("Bearer {}", t)); }
        b.body(Body::from(r#"{"source": "= Hello", "filename": "main.typ"}"#)).unwrap()
    }

    fn req_admin(uri: &str, token: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method("POST").uri(uri);
        if let Some(t) = token { b = b.header(header::AUTHORIZATION, format!("Bearer {}", t)); }
        b.body(Body::empty()).unwrap()
    }
    
    #[tokio::test]
    async fn test_auth_disabled() {
        let mut config = crate::config::Config::load();
        config.auth_token = None;
        config.admin_token = None;
        let state = create_test_state(None); // test helper uses this, need to ensure admin is None
        let mut state = state;
        state.config = std::sync::Arc::new(config);
        let app = create_app(state);
        
        let res = app.clone().oneshot(req_compile_source(None)).await.unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);
        
        let res1b = app.clone().oneshot(req_compile(None)).await.unwrap();
        assert_ne!(res1b.status(), StatusCode::UNAUTHORIZED);
        
        let res2 = app.clone().oneshot(req_admin("/admin/fonts/refresh", None)).await.unwrap();
        assert_ne!(res2.status(), StatusCode::UNAUTHORIZED);
        
        let res2b = app.oneshot(req_admin("/admin/packages/sync-all", None)).await.unwrap();
        assert_ne!(res2b.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_admin_token_only() {
        let mut config = crate::config::Config::load();
        config.auth_token = None;
        config.admin_token = Some("admin_secret".to_string());
        let mut state = create_test_state(None);
        state.config = std::sync::Arc::new(config);
        let app = create_app(state);
        
        let res = app.clone().oneshot(req_compile_source(None)).await.unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);

        let res1b = app.clone().oneshot(req_compile(None)).await.unwrap();
        assert_ne!(res1b.status(), StatusCode::UNAUTHORIZED);
        
        // Check admin token protects refresh endpoint
        let res2 = app.clone().oneshot(req_admin("/admin/fonts/refresh", None)).await.unwrap();
        assert_eq!(res2.status(), StatusCode::UNAUTHORIZED);

        let res2b = app.clone().oneshot(req_admin("/admin/packages/sync-all", None)).await.unwrap();
        assert_eq!(res2b.status(), StatusCode::UNAUTHORIZED);
        
        let res3 = app.clone().oneshot(req_admin("/admin/fonts/refresh", Some("admin_secret"))).await.unwrap();
        assert_ne!(res3.status(), StatusCode::UNAUTHORIZED);

        let res3b = app.oneshot(req_admin("/admin/packages/sync-all", Some("admin_secret"))).await.unwrap();
        assert_ne!(res3b.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_token_only() {
        let mut config = crate::config::Config::load();
        config.auth_token = Some("user_secret".to_string());
        // Simulate the fallback from Config::load()
        config.admin_token = Some("user_secret".to_string());
        
        let mut state = create_test_state(None);
        state.config = std::sync::Arc::new(config);
        let app = create_app(state);
        
        // 1. Compile WITHOUT token SHOULD fail
        let res = app.clone().oneshot(req_compile_source(None)).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        
        let res1b = app.clone().oneshot(req_compile(None)).await.unwrap();
        assert_eq!(res1b.status(), StatusCode::UNAUTHORIZED);
        
        // 2. Compile WITH token SHOULD succeed
        let res2 = app.clone().oneshot(req_compile_source(Some("user_secret"))).await.unwrap();
        assert_ne!(res2.status(), StatusCode::UNAUTHORIZED);

        let res2b = app.clone().oneshot(req_compile(Some("user_secret"))).await.unwrap();
        assert_ne!(res2b.status(), StatusCode::UNAUTHORIZED);
        
        // 3. Admin request WITHOUT token SHOULD fail
        let res3 = app.clone().oneshot(req_admin("/admin/fonts/refresh", None)).await.unwrap();
        assert_eq!(res3.status(), StatusCode::UNAUTHORIZED);
        
        let res3b = app.clone().oneshot(req_admin("/admin/packages/sync-all", None)).await.unwrap();
        assert_eq!(res3b.status(), StatusCode::UNAUTHORIZED);
        
        // 4. Admin request WITH token SHOULD succeed
        let res4 = app.clone().oneshot(req_admin("/admin/fonts/refresh", Some("user_secret"))).await.unwrap();
        assert_ne!(res4.status(), StatusCode::UNAUTHORIZED);

        let res4b = app.oneshot(req_admin("/admin/packages/sync-all", Some("user_secret"))).await.unwrap();
        assert_ne!(res4b.status(), StatusCode::UNAUTHORIZED);
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
        let res = app.clone().oneshot(req_compile_source(Some("user_secret"))).await.unwrap();
        assert_ne!(res.status(), StatusCode::UNAUTHORIZED);

        let res1b = app.clone().oneshot(req_compile(Some("user_secret"))).await.unwrap();
        assert_ne!(res1b.status(), StatusCode::UNAUTHORIZED);
        
        // admin can compile
        let res2 = app.clone().oneshot(req_compile_source(Some("admin_secret"))).await.unwrap();
        assert_ne!(res2.status(), StatusCode::UNAUTHORIZED);

        let res2b = app.clone().oneshot(req_compile(Some("admin_secret"))).await.unwrap();
        assert_ne!(res2b.status(), StatusCode::UNAUTHORIZED);
        
        // user cannot refresh fonts or access admin packages
        let res3 = app.clone().oneshot(req_admin("/admin/fonts/refresh", Some("user_secret"))).await.unwrap();
        assert_eq!(res3.status(), StatusCode::UNAUTHORIZED);

        let res3b = app.oneshot(req_admin("/admin/packages/sync-all", Some("user_secret"))).await.unwrap();
        assert_eq!(res3b.status(), StatusCode::UNAUTHORIZED);
    }
}

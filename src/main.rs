mod font_cache;
mod config;
mod fonts;
mod handlers;
mod middleware;
mod packages;
mod state;
mod world;

use crate::middleware::{admin_auth_middleware, auth_middleware};
use axum::middleware::from_fn_with_state;
use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    routing::{delete, get, post},
};
use config::Config;
use fonts::FontState;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use typst_kit::downloader::SystemDownloader;
use typst_kit::packages::SystemPackages;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

use handlers::{
    admin_info_handler, clear_packages_cache_handler, compile_handler, compile_source_handler,
    list_fonts_handler, list_packages_handler, playground_handler, preload_packages_handler,
    refresh_fonts_handler, sync_all_packages_handler,
};
use state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::compile::compile_handler,
        handlers::compile::compile_source_handler,
        handlers::fonts::list_fonts_handler,
        handlers::fonts::refresh_fonts_handler,
        handlers::packages::list_packages_handler,
        handlers::packages::preload_packages_handler,
        handlers::packages::sync_all_packages_handler,
        handlers::packages::clear_packages_cache_handler,
        handlers::admin::admin_info_handler,
        handlers::admin::sync_mirror_fonts_handler,
    ),
    components(
        schemas(
            handlers::CompileSourceRequest,
            handlers::CompileRequest,
            handlers::ErrorResponse,
            handlers::ErrorInfo,
            handlers::SimpleErrorResponse,
            handlers::FontsResponse,
            handlers::FontInfo,
            handlers::RefreshFontsResponse,
            handlers::PackageInfoResponse,
            handlers::PackagesCacheResponse,
            handlers::PreloadPackagesRequest,
            handlers::SimpleSuccessResponse,
            handlers::AdminInfoResponse,
            handlers::AdminConfigInfo,
            handlers::AdminStatsInfo
        )
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Token")
                        .build(),
                ),
            )
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "typst_api=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load();
    tracing::info!("Starting server on port {}", config.port);
    tracing::info!("Font paths: {:?}", config.font_paths);

    let font_state = Arc::new(tokio::sync::RwLock::new(Arc::new(FontState::new(
        &config.font_paths,
    ))));

    let downloader = SystemDownloader::new("typst-api");
    let packages = Arc::new(SystemPackages::new(downloader));

    let state = AppState {
        config: Arc::new(config.clone()),
        font_state,
        packages,
        startup_time: std::time::Instant::now(),
    };

    // Startup routines for packages
    if let Some(preload_list) = state.config.preload_packages.clone() {
        println!("Preloading {} packages in background...", preload_list.len());
        let packages_store = state.packages.clone();
        tokio::spawn(async move {
            let res = tokio::task::spawn_blocking(move || {
                crate::packages::preload_specific(packages_store, preload_list)
            })
            .await;
            
            if let Ok(Err(e)) = res {
                eprintln!("Error preloading packages: {}", e);
            } else if res.is_ok() {
                println!("Preloading complete.");
            }
        });
    }

    if state.config.cache_all_packages {
        println!("CACHE_ALL_PACKAGES is enabled. Starting background sync task...");
        let packages_store = state.packages.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::packages::sync_missing_packages(packages_store).await {
                eprintln!("Background sync task failed: {}", e);
            } else {
                println!("Background sync task complete.");
            }
        });
    }

    if state.config.cache_all_mirror_fonts {
        println!("CACHE_ALL_133_TYPST_FONTS is enabled. Starting background font sync task...");
        let config_clone = state.config.clone();
        let font_state_clone = state.font_state.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::font_cache::sync_mirror(config_clone.clone()).await {
                eprintln!("Background font sync task failed: {}", e);
            } else {
                println!("Background font sync task complete. Rebuilding FontState...");
                // Rebuild FontState to pick up the newly downloaded fonts
                let new_font_state = Arc::new(FontState::new(&config_clone.font_paths));
                let mut guard = font_state_clone.write().await;
                *guard = new_font_state;
                println!("FontState successfully rebuilt with mirrored fonts.");
            }
        });
    }

    let app = create_app(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub(crate) fn create_app(state: AppState) -> Router {
    let cors_layer = if let Some(cors_origins) = &state.config.cors_allowed_origins {
        if cors_origins == "*" {
            CorsLayer::permissive()
        } else {
            let allowed_patterns: Vec<String> = cors_origins
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let allow_origin =
                tower_http::cors::AllowOrigin::predicate(move |origin: &HeaderValue, _| {
                    if let Ok(origin_str) = origin.to_str() {
                        allowed_patterns.iter().any(|pattern| {
                            origin_str == pattern
                                || origin_str.starts_with(&format!("{}:", pattern))
                        })
                    } else {
                        false
                    }
                });

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
        .layer(ConcurrencyLimitLayer::new(
            state.config.max_concurrent_compilations,
        ))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware));

    let admin_router = Router::new()
        .route("/admin/info", get(admin_info_handler))
        .route("/admin/fonts/refresh", post(refresh_fonts_handler))
        .route("/admin/packages", get(list_packages_handler))
        .route("/admin/packages/preload", post(preload_packages_handler))
        .route("/admin/packages/sync-all", post(sync_all_packages_handler))
        .route("/admin/fonts/sync", post(crate::handlers::sync_mirror_fonts_handler))
        .route(
            "/admin/packages/cache",
            delete(clear_packages_cache_handler),
        )
        .route_layer(from_fn_with_state(state.clone(), admin_auth_middleware));

    Router::new()
        .route("/", get(apex_handler))
        .route("/playground", get(playground_handler))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(|| async { "OK" }))
        .route("/fonts", get(list_fonts_handler))
        .nest("/compile", compile_router)
        .merge(admin_router)
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(state.config.max_payload_size))
        .with_state(state)
}

async fn apex_handler() -> Json<Value> {
    Json(json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "github": env!("CARGO_PKG_REPOSITORY"),
        "docs": "/swagger-ui",
        "openapi": "/api-docs/openapi.json",
        "playground": "/playground",
        "health": "/health"
    }))
}

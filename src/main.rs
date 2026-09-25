mod config;
mod fonts;
mod world;
mod middleware;
mod state;
mod handlers;
mod packages;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    routing::{get, post, delete},
};
use axum::middleware::from_fn_with_state;
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_swagger_ui::SwaggerUi;
use crate::middleware::{auth_middleware, admin_auth_middleware};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::CorsLayer;
use config::Config;
use fonts::FontState;
use std::sync::Arc;
use typst_kit::downloader::SystemDownloader;
use typst_kit::packages::SystemPackages;

use state::AppState;
use handlers::{
    compile_handler, compile_source_handler, list_fonts_handler, refresh_fonts_handler,
    list_packages_handler, preload_packages_handler, sync_all_packages_handler, clear_packages_cache_handler,
};

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
            handlers::SimpleSuccessResponse
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

    // Startup routines for packages
    if let Some(preload_list) = state.config.preload_packages.clone() {
        println!("Preloading {} packages...", preload_list.len());
        let packages_store = state.packages.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = crate::packages::preload_specific(packages_store, preload_list) {
                eprintln!("Error preloading packages: {}", e);
            } else {
                println!("Preloading complete.");
            }
        }).await.unwrap();
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
        .layer(ConcurrencyLimitLayer::new(state.config.max_concurrent_compilations))
        .route_layer(from_fn_with_state(state.clone(), auth_middleware));

    let admin_router = Router::new()
        .route("/admin/fonts/refresh", post(refresh_fonts_handler))
        .route("/admin/packages", get(list_packages_handler))
        .route("/admin/packages/preload", post(preload_packages_handler))
        .route("/admin/packages/sync-all", post(sync_all_packages_handler))
        .route("/admin/packages/cache", delete(clear_packages_cache_handler))
        .route_layer(from_fn_with_state(state.clone(), admin_auth_middleware));

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(|| async { "OK" }))
        .route("/fonts", get(list_fonts_handler))
        .nest("/compile", compile_router)
        .merge(admin_router)
        .layer(cors_layer)
        .layer(DefaultBodyLimit::max(state.config.max_payload_size))
        .with_state(state)
}

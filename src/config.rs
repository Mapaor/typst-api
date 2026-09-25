use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub font_paths: Vec<PathBuf>,
    pub max_payload_size: usize,
    pub compilation_timeout_secs: u64,
    pub auth_token: Option<String>,
    pub admin_token: Option<String>,
    pub cors_allowed_origins: Option<String>,
    pub max_concurrent_compilations: usize,
}

impl Config {
    pub fn load() -> Self {
        let _ = dotenvy::dotenv();

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("PORT must be a valid u16");

        let font_paths = env::var("TYPST_FONT_PATHS")
            .unwrap_or_else(|_| "./fonts".to_string())
            .split(',')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        // Default to 50MB (50 * 1024 * 1024 bytes)
        let max_payload_size = env::var("MAX_PAYLOAD_SIZE")
            .map(|s| {
                s.parse()
                    .expect("MAX_PAYLOAD_SIZE must be a number (bytes)")
            })
            .unwrap_or(50 * 1024 * 1024);

        // Default timeout to 10 seconds
        let compilation_timeout_secs = env::var("COMPILATION_TIMEOUT")
            .map(|s| {
                s.parse()
                    .expect("COMPILATION_TIMEOUT must be a number (seconds)")
            })
            .unwrap_or(10);

        let auth_token = env::var("AUTH_TOKEN").ok().filter(|s| !s.trim().is_empty());

        let admin_token = env::var("ADMIN_TOKEN").ok().filter(|s| !s.trim().is_empty()).or_else(|| auth_token.clone());

        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS").ok().filter(|s| !s.trim().is_empty());

        let max_concurrent_compilations = env::var("MAX_CONCURRENT_COMPILATIONS")
            .map(|s| {
                s.parse()
                    .expect("MAX_CONCURRENT_COMPILATIONS must be a number")
            })
            .unwrap_or(10); // default to 10 active compilations

        Self {
            port,
            font_paths,
            max_payload_size,
            compilation_timeout_secs,
            auth_token,
            admin_token,
            cors_allowed_origins,
            max_concurrent_compilations,
        }
    }
}

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
    pub preload_packages: Option<Vec<String>>,
    pub cache_all_packages: bool,
    pub cors_allowed_origins: Option<String>,
    pub max_concurrent_compilations: usize,
    pub cache_all_mirror_fonts: bool,
    pub fonts_index_url: String,
    pub fonts_cache_dir: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        let _ = dotenvy::dotenv();

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .expect("PORT must be a valid u16");

        let cache_all_mirror_fonts = env::var("CACHE_ALL_133_TYPST_FONTS")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase()
            == "true";

        let fonts_index_url = env::var("FONTS_INDEX_URL").unwrap_or_else(|_| {
            "https://raw.githubusercontent.com/Mapaor/typst-fonts-mirror/main/fonts_mirror_index.json".to_string()
        });

        let fonts_cache_dir = PathBuf::from(
            env::var("FONTS_CACHE_DIR").unwrap_or_else(|_| "./fonts/mirror".to_string())
        );

        let mut font_paths: Vec<PathBuf> = env::var("TYPST_FONT_PATHS")
            .unwrap_or_else(|_| "./fonts".to_string())
            .split(',')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        if cache_all_mirror_fonts && !font_paths.contains(&fonts_cache_dir) {
            font_paths.push(fonts_cache_dir.clone());
        }

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

        let preload_packages = env::var("PRELOAD_PACKAGES")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                s.split(',')
                    .map(|pkg| pkg.trim().to_string())
                    .filter(|pkg| !pkg.is_empty())
                    .collect()
            });

        let cache_all_packages = env::var("CACHE_ALL_PACKAGES")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase()
            == "true";

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
            preload_packages,
            cache_all_packages,
            cors_allowed_origins,
            max_concurrent_compilations,
            cache_all_mirror_fonts,
            fonts_index_url,
            fonts_cache_dir,
        }
    }
}

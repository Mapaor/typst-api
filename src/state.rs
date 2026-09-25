use std::sync::Arc;
use tokio::sync::RwLock;
use typst_kit::packages::SystemPackages;
use crate::config::Config;
use crate::fonts::FontState;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<Config>,
    pub(crate) font_state: Arc<RwLock<Arc<FontState>>>,
    pub(crate) packages: Arc<SystemPackages>,
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use std::sync::OnceLock;
    use typst_kit::downloader::SystemDownloader;

    static SHARED_FONT_STATE: OnceLock<Arc<RwLock<Arc<FontState>>>> = OnceLock::new();
    static SHARED_PACKAGES: OnceLock<Arc<SystemPackages>> = OnceLock::new();

    pub(crate) fn create_test_state(token: Option<String>) -> AppState {
        let config = Config {
            port: 8080,
            font_paths: vec![],
            max_payload_size: 1024 * 1024,
            compilation_timeout_secs: 10,
            auth_token: token,
            admin_token: None,
            cors_allowed_origins: None,
            max_concurrent_compilations: 10,
        };
        
        let font_state = SHARED_FONT_STATE.get_or_init(|| {
            Arc::new(RwLock::new(Arc::new(FontState::new(&[]))))
        }).clone();

        let packages = SHARED_PACKAGES.get_or_init(|| {
            let downloader = SystemDownloader::new("typst-api-test");
            Arc::new(SystemPackages::new(downloader))
        }).clone();

        AppState {
            config: Arc::new(config),
            font_state,
            packages,
        }
    }
}

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

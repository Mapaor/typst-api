pub mod compile;
pub mod fonts;
pub mod packages;
pub mod playground;
pub mod admin;

pub(crate) use compile::{compile_handler, compile_source_handler};
pub(crate) use fonts::{list_fonts_handler, refresh_fonts_handler};
pub(crate) use packages::{
    list_packages_handler, preload_packages_handler, sync_all_packages_handler,
    clear_packages_cache_handler,
};
pub(crate) use playground::playground_handler;
pub(crate) use admin::{admin_info_handler, sync_mirror_fonts_handler};

pub(crate) use compile::{CompileRequest, CompileSourceRequest, ErrorInfo, ErrorResponse};
pub(crate) use fonts::{FontInfo, FontsResponse, RefreshFontsResponse};
pub(crate) use packages::{PackageInfoResponse, PackagesCacheResponse, PreloadPackagesRequest, SimpleSuccessResponse};
pub(crate) use admin::{AdminInfoResponse, AdminConfigInfo, AdminStatsInfo};

use utoipa::ToSchema;

#[derive(serde::Serialize, ToSchema)]
pub(crate) struct SimpleErrorResponse {
    pub error: String,
}

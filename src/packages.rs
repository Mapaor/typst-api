use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use typst::syntax::package::PackageSpec;
use typst_kit::packages::SystemPackages;

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct CachedPackage {
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct CacheInfo {
    pub total_packages: usize,
    pub total_size_bytes: u64,
    pub packages: Vec<CachedPackage>,
}

/// Helper to get the typst preview packages cache path
fn get_cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|dir| dir.join("typst/packages/preview"))
}

/// Calculate the total size of a directory recursively
fn get_dir_size(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let mut size = 0;
    if path.as_ref().is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                size += get_dir_size(entry.path())?;
            } else {
                size += metadata.len();
            }
        }
    }
    Ok(size)
}

/// Scans the local cache directory to build a list of already downloaded packages
pub fn get_cached_packages() -> Result<CacheInfo, String> {
    let cache_dir = get_cache_dir().ok_or("Could not determine cache directory")?;
    
    let mut packages = Vec::new();
    let mut total_size_bytes = 0;

    if cache_dir.exists() && let Ok(entries) = fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(versions) = fs::read_dir(entry.path()) {
                for version_entry in versions.flatten() {
                    let version = version_entry.file_name().to_string_lossy().to_string();
                    if let Ok(size) = get_dir_size(version_entry.path()) {
                        total_size_bytes += size;
                        packages.push(CachedPackage {
                            name: name.clone(),
                            version,
                            size_bytes: size,
                        });
                    }
                }
            }
        }
    }

    Ok(CacheInfo {
        total_packages: packages.len(),
        total_size_bytes,
        packages,
    })
}

/// Preloads a specific list of packages given as strings (e.g. "@preview/cetz:0.3.1")
pub fn preload_specific(
    packages_store: Arc<SystemPackages>,
    specs: Vec<String>,
) -> Result<(), String> {
    for spec_str in specs {
        let spec: PackageSpec = spec_str
            .parse::<PackageSpec>()
            .map_err(|e| e.to_string())?;
        
        // This implicitly downloads the package if it's not cached
        packages_store.obtain(&spec).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Removes the contents of the cache directory
pub fn clear_cache() -> Result<(), String> {
    let cache_dir = get_cache_dir().ok_or("Could not determine cache directory")?;
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Fetches the remote index and downloads all missing packages
pub async fn sync_missing_packages(packages_store: Arc<SystemPackages>) -> Result<(), String> {
    let response = reqwest::get("https://packages.typst.org/preview/index.json")
        .await
        .map_err(|e| format!("Failed to fetch index: {}", e))?;
        
    let remote_packages: Vec<PackageInfo> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse index JSON: {}", e))?;

    let cached = get_cached_packages()?;
    
    for remote_pkg in remote_packages {
        let is_cached = cached
            .packages
            .iter()
            .any(|p| p.name == remote_pkg.name && p.version == remote_pkg.version);

        if !is_cached {
            let spec_str = format!("@preview/{}:{}", remote_pkg.name, remote_pkg.version);
            if let Ok(spec) = spec_str.parse::<PackageSpec>() {
                // Ignore errors on individual packages to continue syncing others
                let _ = packages_store.obtain(&spec);
            }
        }
    }

    Ok(())
}

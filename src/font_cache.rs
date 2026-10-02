use crate::config::Config;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct MirrorManifest {
    pub schema_version: u32,
    pub release: String,
    #[allow(dead_code)]
    pub release_last_updated: String,
    pub fonts: Vec<FontEntry>,
}

#[derive(Debug, Deserialize)]
pub struct FontEntry {
    pub id: String,
    pub asset_url: String,
    pub file_format: String,
    pub asset_sha256: String,
    pub asset_size_bytes: u64,
    #[allow(dead_code)]
    pub asset_last_updated: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub id: String,
    pub manifest_release: String,
    pub source_url: String,
    pub file_format: String,
    pub expected_sha256: String,
    pub verified_sha256: String,
    pub asset_size_bytes: u64,
    pub sync_timestamp: String,
}

pub async fn sync_mirror(config: Arc<Config>) -> Result<(), String> {
    if !config.cache_all_mirror_fonts {
        return Ok(());
    }

    let cache_dir = &config.fonts_cache_dir;
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;
    }

    tracing::info!("Fetching font mirror index from {}", config.fonts_index_url);
    
    let client = Client::new();
    let resp = client
        .get(&config.fonts_index_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch index: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to fetch index, status: {}", resp.status()));
    }

    let manifest: MirrorManifest = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse index JSON: {}", e))?;

    if manifest.schema_version != 1 {
        return Err(format!("Unsupported schema version: {}", manifest.schema_version));
    }

    let mut downloaded = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for font in manifest.fonts {
        match sync_font(&client, cache_dir, &manifest.release, &font).await {
            Ok(true) => downloaded += 1,
            Ok(false) => skipped += 1,
            Err(e) => {
                tracing::error!("Failed to sync font {}: {}", font.id, e);
                failed += 1;
            }
        }
    }

    tracing::info!(
        "Font mirror sync complete. Downloaded: {}, Skipped: {}, Failed: {}",
        downloaded, skipped, failed
    );

    Ok(())
}

async fn sync_font(
    client: &Client,
    cache_dir: &Path,
    release: &str,
    font: &FontEntry,
) -> Result<bool, String> {
    let font_dir = cache_dir.join(&font.id);
    let meta_path = font_dir.join(".metadata.json");

    if font_dir.exists() && meta_path.exists()
        && let Ok(meta_bytes) = fs::read(&meta_path)
        && let Ok(meta) = serde_json::from_slice::<CacheMetadata>(&meta_bytes)
        && meta.manifest_release == release
        && meta.expected_sha256 == font.asset_sha256
        && meta.asset_size_bytes == font.asset_size_bytes
    {
        // Already cached and matches
        return Ok(false);
    }

    tracing::info!("Downloading font {}...", font.id);

    let resp = client
        .get(&font.asset_url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status: {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if bytes.len() as u64 != font.asset_size_bytes {
        return Err(format!(
            "Size mismatch. Expected {}, got {}",
            font.asset_size_bytes,
            bytes.len()
        ));
    }

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hasher.finalize();
    let sha256_hash = hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();

    if sha256_hash != font.asset_sha256 {
        return Err(format!(
            "SHA256 mismatch. Expected {}, got {}",
            font.asset_sha256, sha256_hash
        ));
    }

    let font_id = font.id.clone();
    let file_format = font.file_format.clone();
    
    let temp_dir = cache_dir.join(format!("{}_tmp", font_id));
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|e| format!("Failed to remove old temp dir: {}", e))?;
    }
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let bytes_clone = bytes.to_vec();
    let temp_dir_clone = temp_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let temp_archive = temp_dir_clone.join("archive.tmp");
        fs::write(&temp_archive, &bytes_clone).map_err(|e| format!("Write archive failed: {}", e))?;

        if file_format == "zip" {
            extract_zip(&temp_archive, &temp_dir_clone)?;
        } else if file_format == "tar.gz" {
            extract_tar_gz(&temp_archive, &temp_dir_clone)?;
        } else if file_format == "tar.xz" {
            extract_tar_xz(&temp_archive, &temp_dir_clone)?;
        } else {
            return Err(format!("Unsupported file format: {}", file_format));
        }

        fs::remove_file(&temp_archive).ok();
        
        let mut has_font = false;
        for entry in walkdir(&temp_dir_clone)? {
            let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if ext == "ttf" || ext == "otf" || ext == "ttc" || ext == "otc" {
                has_font = true;
                break;
            }
        }

        if !has_font {
            return Err("No supported font files (.ttf, .otf, .ttc, .otc) found in archive".to_string());
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("Task join failed: {}", e))??;

    if font_dir.exists() {
        fs::remove_dir_all(&font_dir).map_err(|e| format!("Failed to remove old font dir: {}", e))?;
    }
    fs::rename(&temp_dir, &font_dir).map_err(|e| format!("Failed to rename temp dir: {}", e))?;

    let meta = CacheMetadata {
        id: font.id.clone(),
        manifest_release: release.to_string(),
        source_url: font.asset_url.clone(),
        file_format: font.file_format.clone(),
        expected_sha256: font.asset_sha256.clone(),
        verified_sha256: sha256_hash,
        asset_size_bytes: font.asset_size_bytes,
        sync_timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let meta_json = serde_json::to_string_pretty(&meta).unwrap();
    fs::write(meta_path, meta_json).map_err(|e| format!("Failed to write metadata: {}", e))?;

    Ok(true)
}

fn walkdir(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                paths.extend(walkdir(&path)?);
            } else {
                paths.push(path);
            }
        }
    }
    Ok(paths)
}

fn validate_path(path: &Path) -> Result<(), String> {
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => return Err("Path traversal detected".into()),
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return Err("Absolute path detected".into()),
            _ => {}
        }
    }
    Ok(())
}

fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|e| format!("Open zip failed: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Read zip failed: {}", e))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent()
                && !p.exists() 
            {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let tar_gz = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);
    
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.to_path_buf();
        validate_path(&path)?;
        entry.unpack_in(dest_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn extract_tar_xz(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let tar_xz = fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let tar = xz2::read::XzDecoder::new(tar_xz);
    let mut archive = tar::Archive::new(tar);
    
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.to_path_buf();
        validate_path(&path)?;
        entry.unpack_in(dest_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

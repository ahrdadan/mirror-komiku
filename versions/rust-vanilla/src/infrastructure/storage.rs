use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::fs;
use tokio::time::sleep;

use crate::domain::models::ChapterMeta;

pub async fn ensure_cache_layout(cache_root: &Path) -> Result<()> {
    fs::create_dir_all(cache_root.join("pages"))
        .await
        .context("failed to create pages cache dir")?;
    fs::create_dir_all(cache_root.join("assets"))
        .await
        .context("failed to create assets cache dir")?;
    Ok(())
}

pub fn chapter_page_dir(cache_root: &Path, chapter_hash: &str) -> PathBuf {
    cache_root.join("pages").join(chapter_hash)
}

pub fn chapter_assets_dir(cache_root: &Path, chapter_hash: &str) -> PathBuf {
    cache_root.join("assets").join(chapter_hash)
}

pub fn hash_url(url: &str) -> String {
    blake3::hash(url.as_bytes()).to_hex().to_string()
}

pub async fn path_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

pub async fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|x| x.to_str()).unwrap_or("tmp")
    ));
    fs::write(&tmp_path, bytes)
        .await
        .with_context(|| format!("failed writing temp file {:?}", tmp_path))?;
    fs::rename(&tmp_path, path)
        .await
        .with_context(|| format!("failed renaming temp file to {:?}", path))?;
    Ok(())
}

pub async fn read_meta(meta_path: &Path) -> Result<ChapterMeta> {
    let raw = fs::read_to_string(meta_path)
        .await
        .with_context(|| format!("failed to read {:?}", meta_path))?;
    serde_json::from_str(&raw).context("failed to parse meta json")
}

pub async fn wait_for_page(path: &Path, timeout: Duration) -> Result<bool> {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if path_exists(path).await {
            return Ok(true);
        }
        sleep(Duration::from_millis(350)).await;
    }
    Ok(false)
}

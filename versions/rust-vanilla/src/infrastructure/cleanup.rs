use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sysinfo::Disks;
use tokio::fs;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::error;

#[derive(Debug, Clone)]
pub struct CleanupConfig {
    pub cache_root: PathBuf,
    pub max_chapter_count: usize,
    pub interval: Duration,
}

#[derive(Debug, Default)]
pub struct CleanupReport {
    pub before_chapters: usize,
    pub before_bytes: u64,
    pub ttl_removed: usize,
    pub count_removed: usize,
    pub disk_removed: usize,
    pub orphan_removed: usize,
    pub temp_removed: usize,
    pub after_chapters: usize,
    pub after_bytes: u64,
    pub disk_limit_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    chapter_hash: String,
    generated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    page_dir: PathBuf,
    assets_dir: PathBuf,
    total_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct CleanupMeta {
    generated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    #[serde(default)]
    total_bytes: u64,
}

pub async fn run_cleanup_worker(
    cfg: CleanupConfig,
    in_progress: Arc<Mutex<HashSet<String>>>,
) -> Result<()> {
    loop {
        if let Err(err) = run_cleanup_once(&cfg, &in_progress).await {
            error!("cleanup-worker run failed: {err:#}");
        }
        sleep(cfg.interval).await;
    }
}

pub async fn run_cleanup_once(
    cfg: &CleanupConfig,
    in_progress: &Arc<Mutex<HashSet<String>>>,
) -> Result<CleanupReport> {
    let now = Utc::now();
    let mut report = CleanupReport::default();
    let mut entries = collect_cache_entries(cfg, in_progress).await?;
    entries.sort_by_key(|e| e.generated_at);
    let mut entries: VecDeque<CacheEntry> = entries.into();

    report.before_chapters = entries.len();
    report.before_bytes = entries.iter().map(|e| e.total_bytes).sum();

    entries = step_remove_expired(entries, now, &mut report).await;

    step_enforce_max_chapter_count(&mut entries, cfg.max_chapter_count, &mut report).await;

    report.disk_limit_bytes = dynamic_disk_limit_bytes(&cfg.cache_root).await;
    step_enforce_disk_limit(&mut entries, report.disk_limit_bytes, &mut report).await;

    let (orphan_removed, temp_removed) =
        cleanup_orphan_and_temp(cfg, in_progress, &entries).await?;
    report.orphan_removed = orphan_removed;
    report.temp_removed = temp_removed;

    report.after_chapters = entries.len();
    report.after_bytes = entries.iter().map(|e| e.total_bytes).sum();

    Ok(report)
}

async fn collect_cache_entries(
    cfg: &CleanupConfig,
    in_progress: &Arc<Mutex<HashSet<String>>>,
) -> Result<Vec<CacheEntry>> {
    let pages_root = cfg.cache_root.join("pages");
    let assets_root = cfg.cache_root.join("assets");
    let in_progress_set = { in_progress.lock().await.clone() };

    let mut entries = match fs::read_dir(&pages_root).await {
        Ok(rd) => rd,
        Err(_) => return Ok(Vec::new()),
    };

    let mut out = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let Ok(ft) = entry.file_type().await else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }

        let chapter_hash = entry.file_name().to_string_lossy().to_string();
        if in_progress_set.contains(&chapter_hash) {
            continue;
        }

        let page_dir = pages_root.join(&chapter_hash);
        let assets_dir = assets_root.join(&chapter_hash);
        let meta_path = page_dir.join("meta.json");
        let meta = match read_meta(&meta_path).await {
            Ok(meta) => meta,
            Err(err) => {
                error!(
                    "cleanup: skip broken entry hash={} reason=meta_read_failed err={err}",
                    chapter_hash
                );
                continue;
            }
        };

        let total_bytes = if meta.total_bytes > 0 {
            meta.total_bytes
        } else {
            let page_bytes = dir_size_bytes(&page_dir).await.unwrap_or(0);
            let assets_bytes = dir_size_bytes(&assets_dir).await.unwrap_or(0);
            page_bytes.saturating_add(assets_bytes)
        };

        out.push(CacheEntry {
            chapter_hash,
            generated_at: meta.generated_at,
            expires_at: meta.expires_at,
            page_dir,
            assets_dir,
            total_bytes,
        });
    }

    Ok(out)
}

async fn step_remove_expired(
    mut entries: VecDeque<CacheEntry>,
    now: DateTime<Utc>,
    report: &mut CleanupReport,
) -> VecDeque<CacheEntry> {
    let mut kept = VecDeque::with_capacity(entries.len());
    while let Some(entry) = entries.pop_front() {
        if entry.expires_at <= now {
            if delete_cache_entry(&entry, "TTL_EXPIRED").await {
                report.ttl_removed += 1;
            } else {
                kept.push_back(entry);
            }
        } else {
            kept.push_back(entry);
        }
    }
    kept
}

async fn step_enforce_max_chapter_count(
    entries: &mut VecDeque<CacheEntry>,
    max_chapter_count: usize,
    report: &mut CleanupReport,
) {
    while entries.len() > max_chapter_count {
        if let Some(oldest) = entries.pop_front() {
            if delete_cache_entry(&oldest, "MAX_CHAPTER_COUNT").await {
                report.count_removed += 1;
            } else {
                break;
            }
        }
    }
}

async fn step_enforce_disk_limit(
    entries: &mut VecDeque<CacheEntry>,
    disk_limit_bytes: Option<u64>,
    report: &mut CleanupReport,
) {
    let Some(limit) = disk_limit_bytes else {
        return;
    };
    let mut total: u128 = entries.iter().map(|e| e.total_bytes as u128).sum();
    let limit_u128 = limit as u128;

    while total > limit_u128 && !entries.is_empty() {
        if let Some(oldest) = entries.pop_front() {
            if delete_cache_entry(&oldest, "MAX_DISK_SIZE_50PCT_FREE").await {
                report.disk_removed += 1;
                total = total.saturating_sub(oldest.total_bytes as u128);
            } else {
                break;
            }
        }
    }
}

async fn cleanup_orphan_and_temp(
    cfg: &CleanupConfig,
    in_progress: &Arc<Mutex<HashSet<String>>>,
    kept_entries: &VecDeque<CacheEntry>,
) -> Result<(usize, usize)> {
    let in_progress_set = { in_progress.lock().await.clone() };
    let kept_set: HashSet<String> = kept_entries
        .iter()
        .map(|entry| entry.chapter_hash.clone())
        .collect();

    let mut orphan_removed = 0usize;
    let mut temp_removed = 0usize;

    orphan_removed += cleanup_orphan_page_dirs(cfg, &in_progress_set, &kept_set).await?;
    orphan_removed += cleanup_orphan_asset_dirs(cfg, &in_progress_set, &kept_set).await?;
    temp_removed += cleanup_temp_files(&cfg.cache_root).await?;

    Ok((orphan_removed, temp_removed))
}

async fn cleanup_orphan_page_dirs(
    cfg: &CleanupConfig,
    in_progress: &HashSet<String>,
    kept: &HashSet<String>,
) -> Result<usize> {
    let pages_root = cfg.cache_root.join("pages");
    let mut removed = 0usize;
    let mut entries = match fs::read_dir(&pages_root).await {
        Ok(rd) => rd,
        Err(_) => return Ok(0),
    };

    while let Some(entry) = entries.next_entry().await? {
        let Ok(ft) = entry.file_type().await else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        let hash = entry.file_name().to_string_lossy().to_string();
        if in_progress.contains(&hash) || kept.contains(&hash) {
            continue;
        }
        let page_dir = pages_root.join(&hash);
        if remove_dir_if_exists(&page_dir).await.is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

async fn cleanup_orphan_asset_dirs(
    cfg: &CleanupConfig,
    in_progress: &HashSet<String>,
    kept: &HashSet<String>,
) -> Result<usize> {
    let pages_root = cfg.cache_root.join("pages");
    let assets_root = cfg.cache_root.join("assets");
    let mut removed = 0usize;
    let mut entries = match fs::read_dir(&assets_root).await {
        Ok(rd) => rd,
        Err(_) => return Ok(0),
    };

    while let Some(entry) = entries.next_entry().await? {
        let Ok(ft) = entry.file_type().await else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }

        let hash = entry.file_name().to_string_lossy().to_string();
        if in_progress.contains(&hash) {
            continue;
        }

        let assets_dir = assets_root.join(&hash);
        let page_dir = pages_root.join(&hash);
        let page_exists = kept.contains(&hash) || fs::metadata(&page_dir).await.is_ok();
        if !page_exists {
            if remove_dir_if_exists(&assets_dir).await.is_ok() {
                removed += 1;
            }
        }
    }

    Ok(removed)
}

async fn cleanup_temp_files(cache_root: &Path) -> Result<usize> {
    let mut removed = 0usize;
    let mut stack = vec![cache_root.to_path_buf()];

    while let Some(current) = stack.pop() {
        let mut entries = match fs::read_dir(&current).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let Ok(ft) = entry.file_type().await else {
                continue;
            };

            if ft.is_dir() {
                stack.push(path);
                continue;
            }
            if !ft.is_file() {
                continue;
            }

            let is_temp = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".tmp") || n.contains(".tmp."))
                .unwrap_or(false);
            if !is_temp {
                continue;
            }

            if fs::remove_file(&path).await.is_ok() {
                removed += 1;
            }
        }
    }

    Ok(removed)
}

async fn delete_cache_entry(entry: &CacheEntry, _reason: &str) -> bool {
    if let Err(err) = remove_dir_if_exists(&entry.page_dir).await {
        error!(
            "cleanup: failed delete page_dir {:?}: {err}",
            entry.page_dir
        );
        return false;
    }
    if let Err(err) = remove_dir_if_exists(&entry.assets_dir).await {
        error!(
            "cleanup: failed delete assets_dir {:?}: {err}",
            entry.assets_dir
        );
        return false;
    }
    true
}

async fn read_meta(path: &Path) -> Result<CleanupMeta> {
    let raw = fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&raw)?)
}

async fn remove_dir_if_exists(path: &Path) -> Result<()> {
    match fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

async fn dir_size_bytes(root: &Path) -> Result<u64> {
    let mut total: u64 = 0;
    let mut stack = vec![root.to_path_buf()];

    while let Some(current) = stack.pop() {
        let mut entries = match fs::read_dir(&current).await {
            Ok(rd) => rd,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err.into()),
        };

        while let Some(entry) = entries.next_entry().await? {
            let Ok(meta) = entry.metadata().await else {
                continue;
            };
            if meta.is_file() {
                total = total.saturating_add(meta.len());
            } else if meta.is_dir() {
                stack.push(entry.path());
            }
        }
    }

    Ok(total)
}

async fn dynamic_disk_limit_bytes(cache_root: &Path) -> Option<u64> {
    let root = cache_root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let canonical_root = std::fs::canonicalize(&root).unwrap_or(root);
        let disks = Disks::new_with_refreshed_list();

        let mut best: Option<(usize, u64)> = None;
        for disk in disks.list() {
            let mount = disk.mount_point();
            if canonical_root.starts_with(mount) {
                let score = mount.to_string_lossy().len();
                let free = disk.available_space();
                match best {
                    Some((best_score, _)) if best_score >= score => {}
                    _ => best = Some((score, free)),
                }
            }
        }

        if let Some((_, free)) = best {
            return Some(free / 2);
        }
        disks.list().first().map(|d| d.available_space() / 2)
    })
    .await
    .ok()
    .flatten()
}

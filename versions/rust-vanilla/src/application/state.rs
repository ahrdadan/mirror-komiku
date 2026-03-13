use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use tokio::sync::{broadcast, Mutex};

use crate::config::AppConfig;
use crate::domain::models::WsEvent;
use crate::infrastructure::cleanup::CleanupConfig;

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub cache_root: PathBuf,
    pub ttl: Duration,
    pub download_concurrency: usize,
    pub encode_concurrency: usize,
    pub prefetch_depth: usize,
    pub allowed_domains: Option<Vec<String>>,
    pub in_progress: Arc<Mutex<HashSet<String>>>,
    pub ws_channels: Arc<Mutex<HashMap<String, broadcast::Sender<WsEvent>>>>,
    pub ws_init_events: Arc<Mutex<HashMap<String, WsEvent>>>,
    pub ws_prefetch_events: Arc<Mutex<HashMap<String, Vec<WsEvent>>>>,
    pub cleanup_config: CleanupConfig,
}

impl AppState {
    pub fn new(client: Client, cfg: &AppConfig) -> Self {
        Self {
            client,
            cache_root: cfg.cache_root.clone(),
            ttl: cfg.ttl,
            download_concurrency: cfg.download_concurrency,
            encode_concurrency: cfg.encode_concurrency,
            prefetch_depth: cfg.prefetch_depth,
            allowed_domains: cfg.allowed_domains.clone(),
            in_progress: Arc::new(Mutex::new(HashSet::new())),
            ws_channels: Arc::new(Mutex::new(HashMap::new())),
            ws_init_events: Arc::new(Mutex::new(HashMap::new())),
            ws_prefetch_events: Arc::new(Mutex::new(HashMap::new())),
            cleanup_config: CleanupConfig {
                cache_root: cfg.cache_root.clone(),
                max_chapter_count: cfg.max_chapter_count,
                interval: cfg.cleanup_interval,
            },
        }
    }
}

pub async fn try_begin_generation(state: &AppState, chapter_hash: &str) -> bool {
    let mut in_progress = state.in_progress.lock().await;
    if in_progress.contains(chapter_hash) {
        false
    } else {
        in_progress.insert(chapter_hash.to_string());
        true
    }
}

pub async fn end_generation(state: &AppState, chapter_hash: &str) {
    let mut in_progress = state.in_progress.lock().await;
    in_progress.remove(chapter_hash);
}

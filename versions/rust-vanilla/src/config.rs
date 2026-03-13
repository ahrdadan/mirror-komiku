use std::path::PathBuf;
use std::time::Duration;

use crate::domain::models::RunMode;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub port: u16,
    pub ttl: Duration,
    pub max_chapter_count: usize,
    pub download_concurrency: usize,
    pub encode_concurrency: usize,
    pub prefetch_depth: usize,
    pub cleanup_interval: Duration,
    pub cache_root: PathBuf,
    pub allowed_domains: Option<Vec<String>>,
    pub run_mode: RunMode,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = read_env_u16("PORT", 7860);
        let ttl = Duration::from_secs(read_env_u64("CACHE_TTL_SECONDS", 5 * 60 * 60));
        let max_chapter_count = read_env_usize("MAX_CHAPTER_COUNT", 20);
        let download_concurrency = read_env_usize("DOWNLOAD_CONCURRENCY", 4).max(1);
        let encode_concurrency = read_env_usize("ENCODE_CONCURRENCY", 1).max(1);
        let prefetch_depth = read_env_usize("PREFETCH_DEPTH", 3).max(1);
        let cleanup_interval =
            Duration::from_secs(read_env_u64("CLEANUP_INTERVAL_SECONDS", 5 * 60));
        let cache_root = std::env::var("CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("cache"));
        let allowed_domains = parse_allowed_domains(
            std::env::var("ALLOWED_DOMAINS")
                .ok()
                .as_deref()
                .unwrap_or("komiku.org,img.komiku.org"),
        );
        let run_mode = read_run_mode();

        Self {
            bind_addr,
            port,
            ttl,
            max_chapter_count,
            download_concurrency,
            encode_concurrency,
            prefetch_depth,
            cleanup_interval,
            cache_root,
            allowed_domains,
            run_mode,
        }
    }
}

fn read_env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default)
}

fn read_env_u16(name: &str, default: u16) -> u16 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(default)
}

fn read_env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn parse_allowed_domains(input: &str) -> Option<Vec<String>> {
    let domains = input
        .split(',')
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .filter(|d| !d.is_empty())
        .collect::<Vec<_>>();
    if domains.is_empty() {
        None
    } else {
        Some(domains)
    }
}

fn read_run_mode() -> RunMode {
    let arg_mode = std::env::args().nth(1);
    let env_mode = std::env::var("RUN_MODE").ok();
    let raw = arg_mode
        .or(env_mode)
        .unwrap_or_else(|| "all".to_string())
        .to_ascii_lowercase();

    match raw.as_str() {
        "web" => RunMode::Web,
        "worker" => RunMode::Worker,
        _ => RunMode::All,
    }
}

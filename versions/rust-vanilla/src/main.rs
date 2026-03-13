mod application;
mod config;
mod domain;
mod infrastructure;
mod presentation;

use anyhow::{Context, Result};
use reqwest::Client;
use tracing::{info, warn};

use crate::application::state::AppState;
use crate::config::AppConfig;
use crate::domain::models::RunMode;
use crate::infrastructure::cleanup::run_cleanup_worker;
use crate::infrastructure::storage::ensure_cache_layout;
use crate::presentation::http::run_web_server;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cfg = AppConfig::from_env();
    ensure_cache_layout(&cfg.cache_root).await?;

    let client = Client::builder()
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
        )
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("failed to build reqwest client")?;

    let state = AppState::new(client, &cfg);

    info!(
        "run_mode={:?} bind={}:{} download_concurrency={} encode_concurrency={} prefetch_depth={} ttl_secs={} cleanup_interval_secs={}",
        cfg.run_mode,
        cfg.bind_addr,
        cfg.port,
        cfg.download_concurrency,
        cfg.encode_concurrency,
        cfg.prefetch_depth,
        cfg.ttl.as_secs(),
        cfg.cleanup_interval.as_secs()
    );

    match cfg.run_mode {
        RunMode::Web => run_web_server(state, &cfg.bind_addr, cfg.port).await,
        RunMode::Worker => {
            run_cleanup_worker(state.cleanup_config.clone(), state.in_progress.clone()).await
        }
        RunMode::All => {
            let cleanup_cfg = state.cleanup_config.clone();
            let in_progress = state.in_progress.clone();
            tokio::spawn(async move {
                if let Err(err) = run_cleanup_worker(cleanup_cfg, in_progress).await {
                    warn!("cleanup worker stopped: {err:#}");
                }
            });
            run_web_server(state, &cfg.bind_addr, cfg.port).await
        }
    }
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "mirror_komiku=info,info".to_string()),
        )
        .try_init();
}

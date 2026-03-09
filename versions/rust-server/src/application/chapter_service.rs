use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use tokio::fs;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use url::Url;

use crate::application::state::{end_generation, try_begin_generation, AppState};
use crate::application::ws_hub::{ws_drop, ws_emit};
use crate::domain::models::{ChapterMeta, WsEvent};
use crate::domain::parser::parse_chapter_html;
use crate::infrastructure::cleanup::run_cleanup_once;
use crate::infrastructure::html::{build_reader_html, mirror_path_for_url, raw_path_for_url};
use crate::infrastructure::image::convert_to_avif;
use crate::infrastructure::network::{fetch_binary, fetch_html};
use crate::infrastructure::security::validate_url_security;
use crate::infrastructure::storage::{
    chapter_assets_dir, chapter_page_dir, read_meta, wait_for_page, write_atomic,
};

pub async fn spawn_regeneration_if_needed(
    source_url: Url,
    chapter_hash: String,
    state: AppState,
    allow_prefetch: bool,
) {
    if !try_begin_generation(&state, &chapter_hash).await {
        return;
    }

    tokio::spawn(async move {
        if let Err(err) = generate_chapter(source_url, &chapter_hash, &state, allow_prefetch).await {
            warn!("background regeneration failed: {err:#}");
        }
        end_generation(&state, &chapter_hash).await;
    });
}

pub async fn generate_chapter_live_pipeline(
    source_url: Url,
    chapter_hash: String,
    state: &AppState,
) -> Result<()> {
    let started = Instant::now();
    validate_url_security(&source_url, &state.allowed_domains).await?;

    let fetch_started = Instant::now();
    let source_html = fetch_html(&state.client, &source_url).await?;
    info!(
        "chapter={} phase=live_fetch_html ms={}",
        chapter_hash,
        fetch_started.elapsed().as_millis()
    );

    let parse_started = Instant::now();
    let parsed = parse_chapter_html(&source_html, &source_url)
        .with_context(|| format!("failed to parse chapter html for {}", source_url))?;
    info!(
        "chapter={} phase=live_parse_html images={} ms={}",
        chapter_hash,
        parsed.image_urls.len(),
        parse_started.elapsed().as_millis()
    );

    if parsed.image_urls.is_empty() {
        return Err(anyhow!("no chapter images found"));
    }

    ws_emit(
        state,
        &chapter_hash,
        WsEvent::ChapterInit {
            chapter_hash: chapter_hash.clone(),
            title: parsed.title.clone(),
            total_images: parsed.image_urls.len(),
            raw_first_three: parsed
                .image_urls
                .iter()
                .take(3)
                .map(|u| u.to_string())
                .collect(),
            raw_remaining: parsed
                .image_urls
                .iter()
                .skip(3)
                .map(|u| u.to_string())
                .collect(),
            next_mirror_path: parsed.next_url.as_ref().map(mirror_path_for_url),
        },
    )
    .await;

    let assets_dir = chapter_assets_dir(&state.cache_root, &chapter_hash);
    let pages_dir = chapter_page_dir(&state.cache_root, &chapter_hash);
    fs::create_dir_all(&assets_dir).await?;
    fs::create_dir_all(&pages_dir).await?;

    let tail_jobs = parsed
        .image_urls
        .iter()
        .enumerate()
        .skip(3)
        .map(|(idx, url)| (idx + 1, url.clone()))
        .collect::<Vec<_>>();
    let convert_tail_started = Instant::now();
    let mut assets_total_bytes =
        process_selected_images_parallel(&tail_jobs, assets_dir.clone(), chapter_hash.clone(), state, true)
            .await?;
    info!(
        "chapter={} phase=convert_tail count={} ms={}",
        chapter_hash,
        tail_jobs.len(),
        convert_tail_started.elapsed().as_millis()
    );

    let head_jobs = parsed
        .image_urls
        .iter()
        .enumerate()
        .take(3)
        .map(|(idx, url)| (idx + 1, url.clone()))
        .collect::<Vec<_>>();
    let convert_head_started = Instant::now();
    assets_total_bytes = assets_total_bytes.saturating_add(
        process_selected_images_parallel(&head_jobs, assets_dir.clone(), chapter_hash.clone(), state, false)
            .await?,
    );
    info!(
        "chapter={} phase=convert_head count={} ms={}",
        chapter_hash,
        head_jobs.len(),
        convert_head_started.elapsed().as_millis()
    );

    let next_link = parsed.next_url.as_ref().map(mirror_path_for_url);
    let html = build_reader_html(&parsed.title, &chapter_hash, parsed.image_urls.len(), next_link);
    let html_path = pages_dir.join("index.html");
    write_atomic(&html_path, html.as_bytes()).await?;
    let html_bytes = html.len() as u64;

    let now = Utc::now();
    let expires = chrono::Duration::from_std(state.ttl).context("invalid ttl duration")?;
    let mut meta = ChapterMeta {
        source_url: source_url.to_string(),
        next_url: parsed.next_url.as_ref().map(Url::to_string),
        generated_at: now,
        expires_at: now + expires,
        title: parsed.title.clone(),
        image_count: parsed.image_urls.len(),
        total_bytes: 0,
    };
    let meta_path = pages_dir.join("meta.json");
    let mut meta_json = serde_json::to_vec_pretty(&meta)?;
    meta.total_bytes = assets_total_bytes
        .saturating_add(html_bytes)
        .saturating_add(meta_json.len() as u64);
    meta_json = serde_json::to_vec_pretty(&meta)?;
    write_atomic(&meta_path, &meta_json).await?;

    ws_emit(
        state,
        &chapter_hash,
        WsEvent::ChapterDone {
            chapter_hash: chapter_hash.clone(),
        },
    )
    .await;

    prefetch_next_three_after_chapter(parsed.next_url.clone(), &chapter_hash, state).await?;

    trigger_post_generate_cleanup(state.clone());

    info!(
        "chapter={} phase=live_pipeline_done images={} ms={}",
        chapter_hash,
        parsed.image_urls.len(),
        started.elapsed().as_millis()
    );

    Ok(())
}

pub async fn generate_raw_chapter_live_pipeline(
    source_url: Url,
    chapter_hash: String,
    state: &AppState,
) -> Result<()> {
    let started = Instant::now();
    validate_url_security(&source_url, &state.allowed_domains).await?;

    let source_html = fetch_html(&state.client, &source_url).await?;
    let parsed = parse_chapter_html(&source_html, &source_url)
        .with_context(|| format!("failed to parse chapter html for {}", source_url))?;
    if parsed.image_urls.is_empty() {
        return Err(anyhow!("no chapter images found"));
    }

    let mut raw_first_three = Vec::new();
    let mut raw_remaining = Vec::new();
    for (idx, img_url) in parsed.image_urls.iter().enumerate() {
        validate_url_security(img_url, &state.allowed_domains).await?;
        if idx < 3 {
            raw_first_three.push(img_url.to_string());
        } else {
            raw_remaining.push(img_url.to_string());
        }
    }

    ws_emit(
        state,
        &chapter_hash,
        WsEvent::RawChapterInit {
            chapter_hash: chapter_hash.clone(),
            title: parsed.title.clone(),
            total_images: parsed.image_urls.len(),
            raw_first_three,
            raw_remaining,
            next_raw_path: parsed.next_url.as_ref().map(raw_path_for_url),
        },
    )
    .await;

    info!(
        "chapter={} phase=raw_live_pipeline_done images={} ms={}",
        chapter_hash,
        parsed.image_urls.len(),
        started.elapsed().as_millis()
    );

    Ok(())
}

pub async fn generate_chapter(
    source_url: Url,
    chapter_hash: &str,
    state: &AppState,
    allow_prefetch: bool,
) -> Result<()> {
    generate_chapter_without_prefetch(source_url, chapter_hash, state).await?;

    if allow_prefetch {
        let meta_path = chapter_page_dir(&state.cache_root, chapter_hash).join("meta.json");
        let meta = read_meta(&meta_path).await?;
        let next_url = meta.next_url.as_ref().and_then(|u| Url::parse(u).ok());
        prefetch_next_three_after_chapter(next_url, chapter_hash, state).await?;
    }

    Ok(())
}

async fn generate_chapter_without_prefetch(
    source_url: Url,
    chapter_hash: &str,
    state: &AppState,
) -> Result<()> {
    let started = Instant::now();
    validate_url_security(&source_url, &state.allowed_domains).await?;

    info!("generating chapter {} ({})", source_url, chapter_hash);

    let fetch_started = Instant::now();
    let source_html = fetch_html(&state.client, &source_url).await?;
    info!(
        "chapter={} phase=fetch_html ms={}",
        chapter_hash,
        fetch_started.elapsed().as_millis()
    );

    let parse_started = Instant::now();
    let parsed = parse_chapter_html(&source_html, &source_url)
        .with_context(|| format!("failed to parse chapter html for {}", source_url))?;
    info!(
        "chapter={} phase=parse_html images={} ms={}",
        chapter_hash,
        parsed.image_urls.len(),
        parse_started.elapsed().as_millis()
    );

    if parsed.image_urls.is_empty() {
        return Err(anyhow!("no chapter images found"));
    }

    let assets_dir = chapter_assets_dir(&state.cache_root, chapter_hash);
    let pages_dir = chapter_page_dir(&state.cache_root, chapter_hash);
    fs::create_dir_all(&assets_dir).await?;
    fs::create_dir_all(&pages_dir).await?;

    let jobs = parsed
        .image_urls
        .iter()
        .enumerate()
        .map(|(idx, url)| (idx + 1, url.clone()))
        .collect::<Vec<_>>();
    let convert_started = Instant::now();
    let assets_total_bytes =
        process_selected_images_parallel(&jobs, assets_dir.clone(), chapter_hash.to_string(), state, false)
            .await?;
    info!(
        "chapter={} phase=convert_all images={} ms={}",
        chapter_hash,
        jobs.len(),
        convert_started.elapsed().as_millis()
    );

    let next_link = parsed.next_url.as_ref().map(mirror_path_for_url);
    let html = build_reader_html(&parsed.title, chapter_hash, parsed.image_urls.len(), next_link);
    let html_path = pages_dir.join("index.html");
    write_atomic(&html_path, html.as_bytes()).await?;
    let html_bytes = html.len() as u64;

    let now = Utc::now();
    let expires = chrono::Duration::from_std(state.ttl).context("invalid ttl duration")?;
    let mut meta = ChapterMeta {
        source_url: source_url.to_string(),
        next_url: parsed.next_url.as_ref().map(Url::to_string),
        generated_at: now,
        expires_at: now + expires,
        title: parsed.title.clone(),
        image_count: parsed.image_urls.len(),
        total_bytes: 0,
    };
    let meta_path = pages_dir.join("meta.json");
    let mut meta_json = serde_json::to_vec_pretty(&meta)?;
    meta.total_bytes = assets_total_bytes
        .saturating_add(html_bytes)
        .saturating_add(meta_json.len() as u64);
    meta_json = serde_json::to_vec_pretty(&meta)?;
    write_atomic(&meta_path, &meta_json).await?;

    trigger_post_generate_cleanup(state.clone());

    info!(
        "chapter={} phase=chapter_done total_ms={}",
        chapter_hash,
        started.elapsed().as_millis()
    );

    Ok(())
}

pub async fn prefetch_next_three_after_chapter(
    mut next_url: Option<Url>,
    ws_chapter_hash: &str,
    state: &AppState,
) -> Result<()> {
    for _ in 0..state.prefetch_depth {
        let Some(url) = next_url.clone() else {
            break;
        };
        let next_hash = crate::infrastructure::storage::hash_url(url.as_str());

        if try_begin_generation(state, &next_hash).await {
            let result = generate_chapter_without_prefetch(url.clone(), &next_hash, state).await;
            end_generation(state, &next_hash).await;
            if let Err(err) = result {
                warn!("prefetch next chapter failed {}: {err:#}", url);
                break;
            }
        } else {
            let html_path = chapter_page_dir(&state.cache_root, &next_hash).join("index.html");
            let _ = wait_for_page(&html_path, std::time::Duration::from_secs(45)).await;
        }

        let meta_path = chapter_page_dir(&state.cache_root, &next_hash).join("meta.json");
        let meta = match read_meta(&meta_path).await {
            Ok(v) => v,
            Err(_) => break,
        };

        let image_urls = (1..=meta.image_count)
            .map(|i| format!("/assets/{}/{:03}.avif", next_hash, i))
            .collect::<Vec<_>>();

        ws_emit(
            state,
            ws_chapter_hash,
            WsEvent::PrefetchedChapter {
                chapter_hash: next_hash.clone(),
                source_url: meta.source_url.clone(),
                title: meta.title.clone(),
                image_urls,
            },
        )
        .await;

        next_url = meta.next_url.as_ref().and_then(|u| Url::parse(u).ok());
    }
    Ok(())
}

pub async fn prefetch_raw_next_chapters(
    mut next_url: Option<Url>,
    ws_chapter_hash: &str,
    state: &AppState,
    depth: usize,
) -> Result<()> {
    let mut failure_streak = 0usize;

    for _ in 0..depth.max(1) {
        let Some(url) = next_url.clone() else {
            break;
        };

        match prefetch_raw_chapter_once(&url, ws_chapter_hash, state).await {
            Ok(next) => {
                failure_streak = 0;
                next_url = next;
            }
            Err(err) => {
                warn!("raw prefetch failed {}: {err:#}", url);
                failure_streak = failure_streak.saturating_add(1);
                next_url = guess_next_chapter_url(&url);
                if next_url.is_none() || failure_streak >= 2 {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn prefetch_raw_chapter_once(
    url: &Url,
    ws_chapter_hash: &str,
    state: &AppState,
) -> Result<Option<Url>> {
    validate_url_security(url, &state.allowed_domains).await?;
    let source_html = fetch_html(&state.client, url).await?;
    let parsed = parse_chapter_html(&source_html, url)
        .with_context(|| format!("failed to parse chapter html for {}", url))?;
    if parsed.image_urls.is_empty() {
        return Ok(None);
    }

    let mut image_urls = Vec::with_capacity(parsed.image_urls.len());
    for img_url in &parsed.image_urls {
        validate_url_security(img_url, &state.allowed_domains).await?;
        image_urls.push(img_url.to_string());
    }

    ws_emit(
        state,
        ws_chapter_hash,
        WsEvent::RawPrefetchedChapter {
            chapter_hash: format!("raw-{}", crate::infrastructure::storage::hash_url(url.as_str())),
            source_url: url.to_string(),
            title: parsed.title.clone(),
            image_urls,
            next_raw_path: parsed.next_url.as_ref().map(raw_path_for_url),
        },
    )
    .await;

    Ok(parsed.next_url)
}

fn guess_next_chapter_url(current: &Url) -> Option<Url> {
    let path = current.path();
    let lower = path.to_ascii_lowercase();
    let marker = "chapter-";
    let marker_idx = lower.find(marker)?;
    let num_start = marker_idx + marker.len();
    let number_text = lower[num_start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    if number_text.is_empty() {
        return None;
    }
    let number = number_text.parse::<u64>().ok()?;
    let num_end = num_start + number_text.len();

    let mut next_path = String::with_capacity(path.len() + 2);
    next_path.push_str(&path[..num_start]);
    next_path.push_str(&(number + 1).to_string());
    next_path.push_str(&path[num_end..]);

    let mut next_url = current.clone();
    next_url.set_path(&next_path);
    Some(next_url)
}

pub async fn on_live_pipeline_error(state: &AppState, chapter_hash: &str, err: anyhow::Error) {
    error!("live generation failed for {}: {err:#}", chapter_hash);
    ws_emit(
        state,
        chapter_hash,
        WsEvent::Error {
            message: err.to_string(),
        },
    )
    .await;
    ws_drop(state, chapter_hash).await;
    end_generation(state, chapter_hash).await;
}

async fn process_selected_images_parallel(
    jobs: &[(usize, Url)],
    assets_dir: std::path::PathBuf,
    chapter_hash: String,
    state: &AppState,
    emit_ws: bool,
) -> Result<u64> {
    if jobs.is_empty() {
        return Ok(0);
    }

    let queue = Arc::new(Mutex::new(
        jobs.iter().cloned().collect::<VecDeque<(usize, Url)>>(),
    ));
    let download_sem = Arc::new(Semaphore::new(state.download_concurrency.max(1)));
    let encode_sem = Arc::new(Semaphore::new(state.encode_concurrency.max(1)));
    let worker_count = state
        .download_concurrency
        .max(state.encode_concurrency)
        .max(1)
        .min(jobs.len());

    let mut join_set = JoinSet::new();
    for _ in 0..worker_count {
        let queue = Arc::clone(&queue);
        let download_sem = Arc::clone(&download_sem);
        let encode_sem = Arc::clone(&encode_sem);
        let app_state = state.clone();
        let assets_dir = assets_dir.clone();
        let chapter_hash = chapter_hash.clone();

        join_set.spawn(async move {
            let mut subtotal = 0u64;
            loop {
                let next_job = {
                    let mut q = queue.lock().await;
                    q.pop_front()
                };
                let Some((index, img_url)) = next_job else {
                    break;
                };

                let download_permit = download_sem
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| anyhow!("download semaphore closed"))?;
                validate_url_security(&img_url, &app_state.allowed_domains).await?;
                let source_bytes = fetch_binary(&app_state.client, &img_url).await?;
                drop(download_permit);

                let encode_permit = encode_sem
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| anyhow!("encode semaphore closed"))?;
                let avif_bytes = convert_to_avif(source_bytes).await?;
                drop(encode_permit);

                let out_path = assets_dir.join(format!("{:03}.avif", index));
                write_atomic(&out_path, &avif_bytes).await?;
                let bytes = avif_bytes.len() as u64;
                subtotal = subtotal.saturating_add(bytes);

                if emit_ws {
                    ws_emit(
                        &app_state,
                        &chapter_hash,
                        WsEvent::ImageAvif {
                            chapter_hash: chapter_hash.clone(),
                            index,
                            url: format!("/assets/{}/{:03}.avif", chapter_hash, index),
                        },
                    )
                    .await;
                }
            }

            Ok::<u64, anyhow::Error>(subtotal)
        });
    }

    let mut total_bytes = 0u64;
    while let Some(joined) = join_set.join_next().await {
        match joined {
            Ok(Ok(bytes)) => {
                total_bytes = total_bytes.saturating_add(bytes);
            }
            Ok(Err(err)) => return Err(err),
            Err(err) => return Err(anyhow!("image worker task failed: {err}")),
        }
    }

    Ok(total_bytes)
}

fn trigger_post_generate_cleanup(state: AppState) {
    let cleanup_cfg = state.cleanup_config.clone();
    let in_progress = state.in_progress.clone();
    tokio::spawn(async move {
        if let Err(err) = run_cleanup_once(&cleanup_cfg, &in_progress).await {
            warn!("post-generate cleanup failed: {err:#}");
        }
    });
}

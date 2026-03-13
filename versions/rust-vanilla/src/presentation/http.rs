use std::path::Path;
use std::time::{Duration, Instant};

use actix_web::http::{header, StatusCode};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use anyhow::Result;
use chrono::Utc;
use tracing::info;

use crate::application::chapter_service::{
    generate_chapter_live_pipeline, generate_raw_chapter_live_pipeline, on_live_pipeline_error,
    spawn_regeneration_if_needed,
};
use crate::application::state::{end_generation, try_begin_generation, AppState};
use crate::application::ws_hub::ws_drop;
use crate::infrastructure::html::{
    build_landing_html, build_live_raw_reader_html, build_live_reader_html, raw_path_for_url,
};
use crate::infrastructure::storage::{
    chapter_page_dir, hash_url, path_exists, read_meta, wait_for_page,
};
use crate::infrastructure::target::decode_and_validate_target;
use crate::presentation::ws::handle_ws;

pub async fn run_web_server(state: AppState, bind_addr: &str, port: u16) -> Result<()> {
    let addr = format!("{bind_addr}:{port}");
    info!("web listening on http://{addr}");
    let shared_state = web::Data::new(state);
    HttpServer::new(move || {
        App::new()
            .app_data(shared_state.clone())
            .route("/", web::get().to(index))
            .route("/mirror/{target:.*}", web::get().to(handle_mirror))
            .route("/raw/{target:.*}", web::get().to(handle_raw))
            .route("/raw-image/{target:.*}", web::get().to(handle_raw_image))
            .route("/ws/{chapter_hash}", web::get().to(handle_ws))
            .route(
                "/assets/{chapter_hash}/{file_name}",
                web::get().to(serve_asset),
            )
            .route("/{target:.*}", web::get().to(handle_fallback))
    })
    .bind(&addr)?
    .run()
    .await?;
    Ok(())
}

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(build_landing_html())
}

async fn handle_mirror(path: web::Path<String>, state: web::Data<AppState>) -> HttpResponse {
    handle_target_request(path.into_inner(), state.get_ref().clone()).await
}

async fn handle_raw(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> HttpResponse {
    handle_raw_target_request(req, path.into_inner(), state.get_ref().clone()).await
}

async fn handle_fallback(path: web::Path<String>, state: web::Data<AppState>) -> HttpResponse {
    let raw = path.into_inner();
    if raw.is_empty() {
        return HttpResponse::NotFound().body("not found");
    }
    handle_target_request(raw, state.get_ref().clone()).await
}

async fn handle_target_request(raw_target: String, state: AppState) -> HttpResponse {
    let source_url = match decode_and_validate_target(&raw_target, &state.allowed_domains).await {
        Ok(url) => url,
        Err(err) => return bad_request(err),
    };

    let chapter_hash = hash_url(source_url.as_str());
    let page_dir = chapter_page_dir(&state.cache_root, &chapter_hash);
    let html_path = page_dir.join("index.html");
    let meta_path = page_dir.join("meta.json");

    let now = Utc::now();
    let html_exists = path_exists(&html_path).await;
    let meta = read_meta(&meta_path).await.ok();

    if html_exists {
        if let Some(meta) = &meta {
            if meta.expires_at > now {
                return serve_html_file(&html_path, "HIT").await;
            }
        }

        spawn_regeneration_if_needed(
            source_url.clone(),
            chapter_hash.clone(),
            state.clone(),
            true,
        )
        .await;
        return serve_html_file(&html_path, "STALE").await;
    }

    if try_begin_generation(&state, &chapter_hash).await {
        let live_started = Instant::now();
        let live_html = build_live_reader_html(&chapter_hash);

        let bg_state = state.clone();
        let bg_hash = chapter_hash.clone();
        let bg_source_url = source_url.clone();
        tokio::spawn(async move {
            if let Err(err) =
                generate_chapter_live_pipeline(bg_source_url, bg_hash.clone(), &bg_state).await
            {
                on_live_pipeline_error(&bg_state, &bg_hash, err).await;
                return;
            }
            ws_drop(&bg_state, &bg_hash).await;
            end_generation(&bg_state, &bg_hash).await;
        });

        info!(
            "chapter={} phase=live_first_response ms={}",
            chapter_hash,
            live_started.elapsed().as_millis()
        );
        return HttpResponse::Ok()
            .insert_header(("x-cache-status", "MISS_STREAMING"))
            .content_type("text/html; charset=utf-8")
            .body(live_html);
    }

    match wait_for_page(&html_path, Duration::from_secs(45)).await {
        Ok(true) => serve_html_file(&html_path, "WAIT").await,
        Ok(false) => HttpResponse::build(StatusCode::SERVICE_UNAVAILABLE)
            .body("chapter generation in progress, retry shortly"),
        Err(err) => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("failed while waiting for generation: {err}")),
    }
}

async fn handle_raw_target_request(
    req: HttpRequest,
    raw_target: String,
    state: AppState,
) -> HttpResponse {
    let source_url = match decode_and_validate_target(&raw_target, &state.allowed_domains).await {
        Ok(url) => url,
        Err(err) => return bad_request(err),
    };

    let canonical_path = raw_path_for_url(&source_url);
    let incoming_target = req.uri().path().strip_prefix("/raw/").unwrap_or_default();
    let should_redirect = incoming_target.starts_with("http://")
        || incoming_target.starts_with("https://")
        || incoming_target.starts_with("http:/")
        || incoming_target.starts_with("https:/");

    if should_redirect {
        return HttpResponse::build(StatusCode::TEMPORARY_REDIRECT)
            .insert_header((header::LOCATION, canonical_path))
            .finish();
    }

    let chapter_hash = format!("raw-{}", hash_url(source_url.as_str()));
    let live_html = build_live_raw_reader_html(&chapter_hash);

    if try_begin_generation(&state, &chapter_hash).await {
        let bg_state = state.clone();
        let bg_hash = chapter_hash.clone();
        tokio::spawn(async move {
            if let Err(err) =
                generate_raw_chapter_live_pipeline(source_url, bg_hash.clone(), &bg_state).await
            {
                on_live_pipeline_error(&bg_state, &bg_hash, err).await;
                return;
            }
            end_generation(&bg_state, &bg_hash).await;
        });
    }

    HttpResponse::Ok()
        .insert_header(("x-cache-status", "RAW_STREAM"))
        .insert_header((header::CACHE_CONTROL, "no-store, max-age=0"))
        .content_type("text/html; charset=utf-8")
        .body(live_html)
}

async fn handle_raw_image(path: web::Path<String>, state: web::Data<AppState>) -> HttpResponse {
    let image_url =
        match decode_and_validate_target(&path.into_inner(), &state.allowed_domains).await {
            Ok(url) => url,
            Err(err) => return bad_request(err),
        };

    let upstream = match state.client.get(image_url).send().await {
        Ok(resp) => resp,
        Err(err) => {
            return HttpResponse::build(StatusCode::BAD_GATEWAY)
                .body(format!("failed to fetch upstream image: {err}"));
        }
    };

    if !upstream.status().is_success() {
        return HttpResponse::build(StatusCode::BAD_GATEWAY)
            .body(format!("upstream status {}", upstream.status()));
    }

    let content_type = upstream
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let bytes = match upstream.bytes().await {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::build(StatusCode::BAD_GATEWAY)
                .body(format!("failed to read upstream image bytes: {err}"));
        }
    };

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, content_type))
        .insert_header((header::CACHE_CONTROL, "public, max-age=86400"))
        .body(bytes)
}

async fn serve_html_file(path: &Path, cache_status: &str) -> HttpResponse {
    match tokio::fs::read_to_string(path).await {
        Ok(html) => HttpResponse::Ok()
            .insert_header(("x-cache-status", cache_status))
            .insert_header((header::CACHE_CONTROL, "no-store, max-age=0"))
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(err) => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("failed to read cached html: {err}")),
    }
}

async fn serve_asset(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let (chapter_hash, file_name) = path.into_inner();
    if !file_name.ends_with(".avif")
        || file_name.contains('/')
        || file_name.contains('\\')
        || chapter_hash.contains('/')
        || chapter_hash.contains('\\')
    {
        return HttpResponse::build(StatusCode::BAD_REQUEST).body("invalid asset path");
    }

    let path = state
        .cache_root
        .join("assets")
        .join(chapter_hash)
        .join(file_name);

    match tokio::fs::read(path).await {
        Ok(bytes) => HttpResponse::Ok()
            .insert_header((header::CONTENT_TYPE, "image/avif"))
            .insert_header((header::CACHE_CONTROL, "public, max-age=31536000, immutable"))
            .body(bytes),
        Err(_) => HttpResponse::build(StatusCode::NOT_FOUND).body("asset not found"),
    }
}

fn bad_request(err: anyhow::Error) -> HttpResponse {
    HttpResponse::build(StatusCode::BAD_REQUEST).body(err.to_string())
}

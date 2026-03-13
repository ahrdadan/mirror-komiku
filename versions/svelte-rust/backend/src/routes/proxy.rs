use crate::providers;
use crate::proxy::fetch_html;
use crate::utils::base64url;
use crate::utils::errors::ProxyError;
use crate::utils::url_guard;
use crate::AppState;
use actix_web::{get, web, HttpResponse};
use serde::Deserialize;
use tracing::warn;

#[derive(Deserialize)]
pub struct ProxyPath {
    provider: String,
    encoded: String,
}

#[derive(Deserialize)]
pub struct ProxyQuery {
    provider: String,
    u: String,
}

#[get("/api/proxy/{provider}/{encoded}")]
pub async fn proxy_with_path(
    state: web::Data<AppState>,
    path: web::Path<ProxyPath>,
) -> Result<HttpResponse, ProxyError> {
    handle_proxy(state, &path.provider, &path.encoded).await
}

#[get("/api/proxy")]
pub async fn proxy_with_query(
    state: web::Data<AppState>,
    query: web::Query<ProxyQuery>,
) -> Result<HttpResponse, ProxyError> {
    handle_proxy(state, &query.provider, &query.u).await
}

async fn handle_proxy(
    state: web::Data<AppState>,
    provider_id: &str,
    encoded_target: &str,
) -> Result<HttpResponse, ProxyError> {
    let provider = providers::resolve_provider(provider_id).ok_or_else(|| {
        ProxyError::BadRequest(format!("unknown provider '{provider_id}'"))
    })?;

    let decoded = base64url::decode_to_string(encoded_target)?;
    let target = url_guard::validate_target_url(&decoded, provider)?;
    url_guard::enforce_resolved_public_host(&target).await?;

    let fetched = fetch_html(&state.http_client, provider, target).await?;

    Ok(HttpResponse::Ok()
        .insert_header(("content-type", "text/html; charset=utf-8"))
        .insert_header(("cache-control", "no-store"))
        .insert_header(("x-content-type-options", "nosniff"))
        .insert_header(("x-upstream-final-url", fetched.final_url.as_str()))
        .body(fetched.body))
}

pub fn try_build_canonical_route(raw_url: &str) -> Option<String> {
    let parsed = url::Url::parse(raw_url).ok()?;
    let host = parsed.host_str()?;
    let provider = providers::provider_for_host(host)?;
    let encoded = base64url::encode(raw_url);
    Some(format!("/{}/{}", provider.id(), encoded))
}

pub fn parse_raw_path(path: &str) -> Option<String> {
    let trimmed = path.trim_start_matches('/');
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return None;
    }
    Some(trimmed.to_string())
}

pub fn log_raw_path_redirect(path: &str) {
    if let Some(raw_url) = parse_raw_path(path) {
        if let Some(canonical) = try_build_canonical_route(&raw_url) {
            warn!("raw url path detected, canonical should be {}", canonical);
        }
    }
}

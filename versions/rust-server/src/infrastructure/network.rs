use anyhow::{Context, Result};
use reqwest::Client;
use url::Url;

pub async fn fetch_html(client: &Client, url: &Url) -> Result<String> {
    let resp = client
        .get(url.clone())
        .send()
        .await
        .with_context(|| format!("failed to fetch {url}"))?;
    let resp = resp
        .error_for_status()
        .with_context(|| format!("upstream returned error for {url}"))?;
    resp.text().await.context("failed to read html body")
}

pub async fn fetch_binary(client: &Client, url: &Url) -> Result<Vec<u8>> {
    let resp = client
        .get(url.clone())
        .send()
        .await
        .with_context(|| format!("failed to fetch image {url}"))?;
    let resp = resp
        .error_for_status()
        .with_context(|| format!("upstream returned image error for {url}"))?;
    Ok(resp.bytes().await?.to_vec())
}

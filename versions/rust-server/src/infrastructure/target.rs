use anyhow::{Context, Result};
use url::Url;

use crate::infrastructure::security::validate_url_security;

pub async fn decode_and_validate_target(
    raw_target: &str,
    allowed_domains: &Option<Vec<String>>,
) -> Result<Url> {
    let target = decode_target(raw_target);
    let url = Url::parse(&target).with_context(|| format!("invalid target URL: {target}"))?;
    validate_url_security(&url, allowed_domains).await?;
    Ok(url)
}

fn decode_target(raw_target: &str) -> String {
    let trimmed = raw_target.trim();
    let maybe_decoded = percent_encoding::percent_decode_str(trimmed)
        .decode_utf8_lossy()
        .to_string();
    fix_slash_after_scheme(&maybe_decoded)
}

fn fix_slash_after_scheme(url: &str) -> String {
    if url.starts_with("https:/") && !url.starts_with("https://") {
        return url.replacen("https:/", "https://", 1);
    }
    if url.starts_with("http:/") && !url.starts_with("http://") {
        return url.replacen("http:/", "http://", 1);
    }
    url.to_string()
}

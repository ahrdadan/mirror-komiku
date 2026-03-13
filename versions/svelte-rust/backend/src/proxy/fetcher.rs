use crate::providers::Provider;
use crate::utils::errors::ProxyError;
use crate::utils::url_guard;
use futures_util::StreamExt;
use reqwest::header::LOCATION;
use reqwest::{Client, Url};

const MAX_REDIRECTS: usize = 5;
const MAX_HTML_BYTES: usize = 4 * 1024 * 1024;

pub struct FetchResult {
    pub body: String,
    pub final_url: Url,
}

pub async fn fetch_html(
    client: &Client,
    provider: &dyn Provider,
    target_url: Url,
) -> Result<FetchResult, ProxyError> {
    let mut current = target_url;

    for hop in 0..=MAX_REDIRECTS {
        let response = client
            .get(current.clone())
            .header("accept", "text/html,application/xhtml+xml")
            .send()
            .await
            .map_err(ProxyError::from_reqwest)?;

        let status = response.status();
        if status.is_redirection() {
            if hop == MAX_REDIRECTS {
                return Err(ProxyError::BadGateway(
                    "redirect limit exceeded".to_string(),
                ));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .ok_or_else(|| ProxyError::BadGateway("redirect missing location".to_string()))?
                .to_str()
                .map_err(|_| ProxyError::BadGateway("invalid redirect location".to_string()))?;
            let next = current
                .join(location)
                .map_err(|_| ProxyError::BadGateway("failed to resolve redirect".to_string()))?;
            url_guard::validate_target_url(next.as_str(), provider)?;
            url_guard::enforce_resolved_public_host(&next).await?;
            current = next;
            continue;
        }

        if !status.is_success() {
            return Err(ProxyError::UpstreamStatus(status.as_u16()));
        }

        if let Some(length) = response.content_length() {
            if length as usize > MAX_HTML_BYTES {
                return Err(ProxyError::PayloadTooLarge(format!(
                    "upstream payload exceeds {} bytes",
                    MAX_HTML_BYTES
                )));
            }
        }

        let mut received = 0usize;
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(ProxyError::from_reqwest)?;
            received += chunk.len();
            if received > MAX_HTML_BYTES {
                return Err(ProxyError::PayloadTooLarge(format!(
                    "upstream payload exceeds {} bytes",
                    MAX_HTML_BYTES
                )));
            }
            body.extend_from_slice(&chunk);
        }

        let text = String::from_utf8_lossy(&body).into_owned();
        return Ok(FetchResult {
            body: text,
            final_url: current,
        });
    }

    Err(ProxyError::BadGateway(
        "failed to fetch html from upstream".to_string(),
    ))
}

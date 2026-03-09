use std::net::IpAddr;

use anyhow::{anyhow, Context, Result};
use url::Url;

pub async fn validate_url_security(url: &Url, allowed_domains: &Option<Vec<String>>) -> Result<()> {
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(anyhow!("only http/https URLs are allowed")),
    }

    let host = url
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| anyhow!("URL host is required"))?;
    if host == "localhost" {
        return Err(anyhow!("localhost is not allowed"));
    }

    if let Some(domains) = allowed_domains {
        let allowed = domains
            .iter()
            .any(|d| host == *d || host.ends_with(&format!(".{d}")));
        if !allowed {
            return Err(anyhow!("host is not in allowed domain list"));
        }
    }

    let port = url.port_or_known_default().unwrap_or(80);
    let addrs = tokio::net::lookup_host((host.as_str(), port))
        .await
        .with_context(|| format!("dns lookup failed for host {host}"))?
        .collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err(anyhow!("host has no DNS records"));
    }

    for addr in addrs {
        if !is_public_ip(addr.ip()) {
            return Err(anyhow!("blocked private/local target address"));
        }
    }

    Ok(())
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !v4.is_private()
                && !v4.is_loopback()
                && !v4.is_link_local()
                && !v4.is_broadcast()
                && !v4.is_documentation()
                && !v4.is_unspecified()
                && !v4.is_multicast()
                && v4.octets()[0] != 0
        }
        IpAddr::V6(v6) => {
            let seg = v6.segments();
            let is_documentation = seg[0] == 0x2001 && seg[1] == 0x0db8;
            !v6.is_loopback()
                && !v6.is_unspecified()
                && !v6.is_multicast()
                && !v6.is_unique_local()
                && !v6.is_unicast_link_local()
                && !is_documentation
        }
    }
}

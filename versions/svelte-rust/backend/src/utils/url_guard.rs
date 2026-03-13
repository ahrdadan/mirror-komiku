use crate::providers::Provider;
use crate::utils::errors::ProxyError;
use std::net::{IpAddr, Ipv4Addr};
use tokio::net::lookup_host;
use url::Url;

pub fn validate_target_url(raw_url: &str, provider: &dyn Provider) -> Result<Url, ProxyError> {
    let target = Url::parse(raw_url)
        .map_err(|_| ProxyError::BadRequest("invalid target url".to_string()))?;

    if target.scheme() != "http" && target.scheme() != "https" {
        return Err(ProxyError::BadRequest(
            "only http/https schemes are allowed".to_string(),
        ));
    }

    let host = target
        .host_str()
        .ok_or_else(|| ProxyError::BadRequest("missing target host".to_string()))?;
    let lower_host = host.to_ascii_lowercase();

    if is_local_or_private_host_name(&lower_host) {
        return Err(ProxyError::Forbidden(
            "private or local targets are blocked".to_string(),
        ));
    }

    if !provider.matches_host(&lower_host) {
        return Err(ProxyError::Forbidden(
            "target host does not match provider domain".to_string(),
        ));
    }

    if let Ok(ip) = lower_host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return Err(ProxyError::Forbidden(
                "private or local ip targets are blocked".to_string(),
            ));
        }
    }

    Ok(target)
}

pub async fn enforce_resolved_public_host(target: &Url) -> Result<(), ProxyError> {
    let host = target
        .host_str()
        .ok_or_else(|| ProxyError::BadRequest("missing target host".to_string()))?;
    let port = target.port_or_known_default().unwrap_or(80);

    let entries = lookup_host((host, port))
        .await
        .map_err(|_| ProxyError::BadGateway("dns lookup failed".to_string()))?;

    for entry in entries {
        if is_private_ip(&entry.ip()) {
            return Err(ProxyError::Forbidden(
                "resolved ip belongs to private/local network".to_string(),
            ));
        }
    }

    Ok(())
}

fn is_local_or_private_host_name(host: &str) -> bool {
    if host == "localhost" || host == "0.0.0.0" || host == "::1" {
        return true;
    }

    if host.ends_with(".localhost") {
        return true;
    }

    false
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(*v4),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
        }
    }
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    let a = octets[0];
    let b = octets[1];

    if a == 10 || a == 127 {
        return true;
    }
    if a == 169 && b == 254 {
        return true;
    }
    if a == 172 && (16..=31).contains(&b) {
        return true;
    }
    if a == 192 && b == 168 {
        return true;
    }
    if ip == Ipv4Addr::new(0, 0, 0, 0) {
        return true;
    }

    false
}

pub mod komiku;

pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn matches_host(&self, host: &str) -> bool;
}

static KOMIKU_PROVIDER: komiku::KomikuProvider = komiku::KomikuProvider;

pub fn resolve_provider(provider_id: &str) -> Option<&'static dyn Provider> {
    match provider_id {
        "komiku" => Some(&KOMIKU_PROVIDER),
        _ => None,
    }
}

pub fn provider_for_host(host: &str) -> Option<&'static dyn Provider> {
    let lower = host.to_ascii_lowercase();
    if KOMIKU_PROVIDER.matches_host(&lower) {
        return Some(&KOMIKU_PROVIDER);
    }
    None
}

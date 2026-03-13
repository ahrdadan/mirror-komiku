use crate::providers::Provider;

pub struct KomikuProvider;

impl Provider for KomikuProvider {
    fn id(&self) -> &'static str {
        "komiku"
    }

    fn matches_host(&self, host: &str) -> bool {
        host == "komiku.org" || host.ends_with(".komiku.org")
    }
}

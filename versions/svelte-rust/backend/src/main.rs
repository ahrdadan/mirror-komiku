mod providers;
mod proxy;
mod routes;
mod utils;

use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use reqwest::redirect::Policy;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub http_client: reqwest::Client,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_tracing();

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let timeout_secs = env_u64("UPSTREAM_TIMEOUT_SECS", 12);

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(Policy::none())
        .user_agent("mirror-komiku-proxy/0.1")
        .build()
        .expect("failed to build reqwest client");

    let state = AppState { http_client };
    info!("starting backend at http://{}", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allowed_methods(["GET", "OPTIONS"])
            .max_age(86_400);

        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::default())
            .wrap(cors)
            .configure(routes::configure)
    })
    .bind(bind_addr)?
    .run()
    .await
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn env_u64(key: &str, fallback: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(fallback)
}

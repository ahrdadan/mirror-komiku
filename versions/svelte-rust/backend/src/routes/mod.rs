mod health;
mod proxy;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health::health);
    cfg.service(proxy::proxy_with_path);
    cfg.service(proxy::proxy_with_query);
}

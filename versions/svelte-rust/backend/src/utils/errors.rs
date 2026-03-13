use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    BadGateway(String),
    #[error("upstream status: {0}")]
    UpstreamStatus(u16),
    #[error("{0}")]
    PayloadTooLarge(String),
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    error: &'a str,
    message: String,
}

impl ProxyError {
    pub fn from_reqwest(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return Self::BadGateway("upstream timeout".to_string());
        }
        Self::BadGateway(format!("upstream request failed: {err}"))
    }
}

impl ResponseError for ProxyError {
    fn status_code(&self) -> StatusCode {
        match self {
            ProxyError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ProxyError::Forbidden(_) => StatusCode::FORBIDDEN,
            ProxyError::BadGateway(_) => StatusCode::BAD_GATEWAY,
            ProxyError::UpstreamStatus(_) => StatusCode::BAD_GATEWAY,
            ProxyError::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let (error_kind, message) = match self {
            ProxyError::BadRequest(message) => ("bad_request", message.clone()),
            ProxyError::Forbidden(message) => ("forbidden", message.clone()),
            ProxyError::BadGateway(message) => ("bad_gateway", message.clone()),
            ProxyError::UpstreamStatus(code) => (
                "upstream_status",
                format!("upstream responded with status {code}"),
            ),
            ProxyError::PayloadTooLarge(message) => ("payload_too_large", message.clone()),
        };

        HttpResponse::build(self.status_code())
            .insert_header(("content-type", "application/json; charset=utf-8"))
            .json(ErrorPayload {
                error: error_kind,
                message,
            })
    }
}

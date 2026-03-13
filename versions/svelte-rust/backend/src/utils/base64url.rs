use crate::utils::errors::ProxyError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

pub fn encode(input: &str) -> String {
    URL_SAFE_NO_PAD.encode(input.as_bytes())
}

pub fn decode_to_string(input: &str) -> Result<String, ProxyError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(input.as_bytes())
        .map_err(|_| ProxyError::BadRequest("invalid base64url payload".to_string()))?;
    String::from_utf8(bytes)
        .map_err(|_| ProxyError::BadRequest("decoded payload is not utf-8".to_string()))
}

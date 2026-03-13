use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

pub fn encode_base64url(input: &str) -> String {
    URL_SAFE_NO_PAD.encode(input.as_bytes())
}

pub fn decode_base64url(input: &str) -> Result<String, String> {
    let decoded = URL_SAFE_NO_PAD
        .decode(input.as_bytes())
        .map_err(|e| format!("decode error: {e}"))?;
    String::from_utf8(decoded).map_err(|e| format!("utf8 error: {e}"))
}

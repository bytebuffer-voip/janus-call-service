use axum::http::header::COOKIE;
use axum::http::HeaderMap;

pub fn get_token_from_cookies(headers: &HeaderMap) -> Option<String> {
    let raw_cookie = headers
        .get(COOKIE)
        .and_then(|val| val.to_str().ok())
        .unwrap_or("");
    for cookie_str in raw_cookie.split(';') {
        let cookie_str = cookie_str.trim();
        if let Some((key, value)) = cookie_str.split_once('=') {
            match key {
                "t" => {
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    None
}

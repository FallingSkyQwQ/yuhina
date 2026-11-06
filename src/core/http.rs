use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{ErrorCode, Result, YuhinaError};

pub const USER_AGENT: &str = "Yuhina/0.2 (+https://github.com/FallingSkyQwQ/yuhina)";
const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn json(&self) -> Result<Value> {
        serde_json::from_str(&self.body)
            .map_err(|error| YuhinaError::new(ErrorCode::Parse, error.to_string()))
    }
}

/// A small blocking HTTP client. Every call runs on a background executor
/// thread; nothing here touches the UI thread.
pub fn request(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<(&str, Vec<u8>)>,
) -> Result<HttpResponse> {
    let mut request = ureq::request(method, url)
        .set("User-Agent", USER_AGENT)
        .timeout(TIMEOUT);
    for (name, value) in headers {
        request = request.set(name, value);
    }
    let result = match body {
        Some((content_type, bytes)) => {
            request = request.set("Content-Type", content_type);
            request.send_bytes(&bytes)
        }
        None => request.call(),
    };

    match result {
        Ok(response) => {
            let status = response.status();
            let body = response.into_string().unwrap_or_default();
            Ok(HttpResponse { status, body })
        }
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            Ok(HttpResponse { status, body })
        }
        Err(ureq::Error::Transport(transport)) => {
            Err(YuhinaError::new(ErrorCode::Network, transport.to_string()))
        }
    }
}

pub fn get_json<T: DeserializeOwned>(url: &str) -> Result<T> {
    let response = request("GET", url, &[("Accept", "application/json")], None)?;
    if !response.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::Http,
            format!("The request to {} failed ({}).", url, response.status),
        ));
    }
    serde_json::from_str(&response.body)
        .map_err(|error| YuhinaError::new(ErrorCode::Parse, error.to_string()))
}

pub fn get_json_value(url: &str) -> Result<Value> {
    get_json(url)
}

pub fn get_text(url: &str) -> Result<String> {
    let response = request("GET", url, &[("Accept", "*/*")], None)?;
    if !response.is_success() {
        return Err(YuhinaError::new(
            ErrorCode::Http,
            format!("The request to {} failed ({}).", url, response.status),
        ));
    }
    Ok(response.body)
}

pub fn post_form(url: &str, fields: &[(&str, &str)]) -> Result<HttpResponse> {
    let body = form_encode(fields);
    request(
        "POST",
        url,
        &[
            ("Accept", "application/json"),
            ("Content-Type", "application/x-www-form-urlencoded"),
        ],
        Some(("application/x-www-form-urlencoded", body.into_bytes())),
    )
}

pub fn post_json(url: &str, value: &Value) -> Result<HttpResponse> {
    post_json_with(url, value, &[])
}

pub fn post_json_with(url: &str, value: &Value, headers: &[(&str, &str)]) -> Result<HttpResponse> {
    let mut all = vec![("Accept", "application/json")];
    all.extend_from_slice(headers);
    let body = serde_json::to_vec(value)?;
    request("POST", url, &all, Some(("application/json", body)))
}

pub fn get_json_with_headers(url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse> {
    request("GET", url, headers, None)
}

pub fn form_encode(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                percent_encoding::utf8_percent_encode(key, percent_encoding::NON_ALPHANUMERIC),
                percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

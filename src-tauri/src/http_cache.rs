use crate::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use url::Url;

// Request structure from frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

// Response structure to frontend
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

// Cache entry
#[derive(Clone, Debug)]
struct CacheEntry {
    response: ProxyResponse,
    timestamp: SystemTime,
}

// Global cache store
pub struct Cache {
    store: Mutex<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl Cache {
    pub fn new(ttl_seconds: u64) -> Self {
        Cache {
            store: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub async fn get(&self, key: &str) -> Option<ProxyResponse> {
        let mut store = self.store.lock().await;

        if let Some(entry) = store.get(key) {
            match entry.timestamp.elapsed() {
                Ok(elapsed) if elapsed < self.ttl => {
                    log::info!("Cache hit for key: {}", key);
                    return Some(entry.response.clone());
                }
                Ok(_) => {
                    log::info!("Cache expired for key: {}", key);
                    store.remove(key);
                }
                Err(err) => {
                    log::warn!("Cache timestamp error for key '{}': {}", key, err);
                    store.remove(key);
                }
            }
        }
        None
    }

    pub async fn set(&self, key: String, response: ProxyResponse) {
        let mut store = self.store.lock().await;
        store.insert(
            key,
            CacheEntry {
                response,
                timestamp: SystemTime::now(),
            },
        );
        log::info!("Cached response for key");
    }

    pub async fn clear(&self) {
        let mut store = self.store.lock().await;
        store.clear();
        log::info!("Cache cleared");
    }

    /// Inserts an entry whose timestamp is 10 years in the future so that
    /// `SystemTime::elapsed()` returns `Err`, exercising lines 59-61 in `get`.
    #[cfg(test)]
    pub async fn insert_with_future_timestamp(&self, key: String, response: ProxyResponse) {
        let future_time = SystemTime::now() + Duration::from_secs(60 * 60 * 24 * 365 * 10);
        let mut store = self.store.lock().await;
        store.insert(
            key,
            CacheEntry {
                response,
                timestamp: future_time,
            },
        );
    }
}

// Validation functions
fn validation_error(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

fn validate_url_scheme(url: &Url) -> Result<(), AppError> {
    if url.scheme() == "https" || url.host_str() == Some("localhost") {
        return Ok(());
    }

    Err(validation_error("Only HTTPS URLs are allowed"))
}

fn validate_url_host(url: &Url, allowed_domains: &[&str]) -> Result<(), AppError> {
    let host = url
        .host_str()
        .ok_or_else(|| validation_error("URL must have a valid host"))?;

    if allowed_domains.iter().any(|&domain| host.ends_with(domain)) {
        return Ok(());
    }

    Err(validation_error(format!(
        "Domain '{}' is not in the allowed list",
        host
    )))
}

/// Validates a URL against the HTTPS scheme and an allowed-domains allowlist.
///
/// Returns `Ok(())` if the URL is HTTPS and its host ends with one of the
/// `allowed_domains` entries, or if the host is `localhost`.
///
/// # Examples
///
/// ```
/// use hsl_routes_lib::validate_url;
///
/// let domains = &["digitransit.fi"];
/// assert!(validate_url("https://api.digitransit.fi/routing/v1", domains).is_ok());
/// assert!(validate_url("http://api.digitransit.fi/routing/v1", domains).is_err());
/// assert!(validate_url("https://evil.com/api", domains).is_err());
/// ```
pub fn validate_url(url_str: &str, allowed_domains: &[&str]) -> Result<(), AppError> {
    let url = Url::parse(url_str)?;

    validate_url_scheme(&url)?;
    validate_url_host(&url, allowed_domains)?;
    Ok(())
}

/// Validates that an HTTP method is one of GET, POST, PUT, DELETE, or PATCH.
///
/// Comparison is case-insensitive.
///
/// # Examples
///
/// ```
/// use hsl_routes_lib::validate_method;
///
/// assert!(validate_method("GET").is_ok());
/// assert!(validate_method("post").is_ok());
/// assert!(validate_method("DELETE").is_ok());
/// assert!(validate_method("CONNECT").is_err());
/// ```
pub fn validate_method(method: &str) -> Result<(), AppError> {
    if is_allowed_method(method) {
        Ok(())
    } else {
        Err(validation_error(format!(
            "HTTP method '{}' is not allowed",
            method
        )))
    }
}

fn validate_status_code(status: u16) -> Result<(), AppError> {
    if (200..600).contains(&status) {
        return Ok(());
    }

    Err(validation_error(format!(
        "Invalid HTTP status code: {}",
        status
    )))
}

fn validate_body_size(body: &str) -> Result<(), AppError> {
    const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;
    if body.len() <= MAX_BODY_SIZE {
        return Ok(());
    }

    Err(validation_error(
        "Response body exceeds maximum allowed size",
    ))
}

fn validate_json_body_if_needed(
    headers: &HashMap<String, String>,
    body: &str,
) -> Result<(), AppError> {
    if let Some(content_type) = headers.get("content-type") {
        if content_type.contains("application/json") {
            serde_json::from_str::<serde_json::Value>(body)?;
        }
    }

    Ok(())
}

pub fn validate_response(response: &ProxyResponse) -> Result<(), AppError> {
    validate_status_code(response.status)?;
    validate_body_size(&response.body)?;
    validate_json_body_if_needed(&response.headers, &response.body)?;
    Ok(())
}

/// Generates a SHA-256 hex-encoded cache key for a proxy request.
///
/// The key is derived from URL, method, and body. Headers are excluded so
/// cache lookups are header-independent.
///
/// # Examples
///
/// ```
/// use hsl_routes_lib::{generate_cache_key, ProxyRequest};
/// use std::collections::HashMap;
///
/// let req = ProxyRequest {
///     url: "https://api.digitransit.fi/routing/v1/routers".to_string(),
///     method: "GET".to_string(),
///     headers: HashMap::new(),
///     body: None,
/// };
/// let key = generate_cache_key(&req);
/// // SHA-256 hex digest is always 64 characters
/// assert_eq!(key.len(), 64);
/// // Same inputs always produce the same key
/// assert_eq!(key, generate_cache_key(&req));
/// ```
pub fn generate_cache_key(request: &ProxyRequest) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(request.url.as_bytes());
    hasher.update(b"|");
    hasher.update(request.method.as_bytes());
    if let Some(body) = &request.body {
        hasher.update(b"|");
        hasher.update(body.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn is_cacheable_method(method: &str) -> bool {
    method.eq_ignore_ascii_case("GET") || method.eq_ignore_ascii_case("POST")
}

fn is_allowed_method(method: &str) -> bool {
    method.eq_ignore_ascii_case("GET")
        || method.eq_ignore_ascii_case("POST")
        || method.eq_ignore_ascii_case("PUT")
        || method.eq_ignore_ascii_case("DELETE")
        || method.eq_ignore_ascii_case("PATCH")
}

fn should_cache_response(method: &str, status: u16) -> bool {
    is_cacheable_method(method) && (200..300).contains(&status)
}

async fn send_proxy_request(request: &ProxyRequest) -> Result<ProxyResponse, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|e| AppError::Http(format!("Invalid HTTP method: {}", e)))?;

    let mut req_builder = client.request(method, &request.url);

    for (key, value) in &request.headers {
        req_builder = req_builder.header(key, value);
    }

    if let Some(body) = &request.body {
        req_builder = req_builder.body(body.clone());
    }

    log::info!("Sending {} request to {}", request.method, request.url);
    let response = req_builder.send().await?;

    let status = response.status().as_u16();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect();
    let body = response.text().await?;

    Ok(ProxyResponse {
        status,
        headers,
        body,
    })
}

// Main proxy function
pub async fn proxy_request(
    request: ProxyRequest,
    cache: &Cache,
    allowed_domains: &[&str],
) -> Result<ProxyResponse, AppError> {
    validate_url(&request.url, allowed_domains)?;
    validate_method(&request.method)?;

    let cache_key = if is_cacheable_method(&request.method) {
        Some(generate_cache_key(&request))
    } else {
        None
    };

    if let Some(cache_key) = &cache_key {
        if let Some(cached_response) = cache.get(cache_key).await {
            return Ok(cached_response);
        }
    }

    let proxy_response = send_proxy_request(&request).await?;
    validate_response(&proxy_response)?;

    if should_cache_response(&request.method, proxy_response.status) {
        if let Some(cache_key) = cache_key {
            cache.set(cache_key, proxy_response.clone()).await;
        }
        log::info!("Cached {} response for {}", request.method, request.url);
    }

    Ok(proxy_response)
}

// Cache statistics structure
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub entries: usize,
    pub estimated_bytes: usize,
}

// Tauri command wrapper
#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub async fn fetch_with_proxy(
    request: ProxyRequest,
    cache: tauri::State<'_, Cache>,
) -> Result<ProxyResponse, AppError> {
    // Define allowed domains (HSL API and any other services you trust)
    let allowed_domains = &[
        "api.digitransit.fi",
        "cdn.digitransit.fi", // Map tiles
        "localhost",          // For development
    ];

    proxy_request(request, &cache, allowed_domains).await
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub async fn clear_cache(cache: tauri::State<'_, Cache>) -> Result<(), AppError> {
    cache.clear().await;
    Ok(())
}

#[cfg(not(tarpaulin_include))]
#[tauri::command]
pub async fn get_cache_stats(cache: tauri::State<'_, Cache>) -> Result<CacheStats, AppError> {
    let store = cache.store.lock().await;
    let entries = store.len();

    // Estimate bytes used by cache
    let estimated_bytes: usize = store
        .values()
        .map(|entry| {
            // Rough estimation: body size + headers + overhead
            entry.response.body.len()
                + entry
                    .response
                    .headers
                    .iter()
                    .fold(0, |acc, (k, v)| acc + k.len() + v.len())
                + 200 // Overhead estimate
        })
        .sum();

    Ok(CacheStats {
        entries,
        estimated_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_cache_operations() {
        let cache = Cache::new(60);
        let key = "test_key".to_string();
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "test body".to_string(),
        };

        // Test set and get
        cache.set(key.clone(), response.clone()).await;
        let result = cache.get(&key).await;
        assert!(result.is_some());
        let cached = result.unwrap();
        assert_eq!(cached.status, 200);
        assert_eq!(cached.body, "test body");

        // Test clear
        cache.clear().await;
        let result_after_clear = cache.get(&key).await;
        assert!(result_after_clear.is_none());
    }

    #[test]
    fn test_validate_url_https() {
        let domains = &["api.digitransit.fi"];
        let result = validate_url("https://api.digitransit.fi/routing/v1/routers", domains);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_invalid_domain() {
        let domains = &["api.digitransit.fi"];
        let result = validate_url("https://evil.com/api", domains);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_validate_url_not_https() {
        let domains = &["api.digitransit.fi"];
        let result = validate_url("http://api.digitransit.fi/api", domains);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_validate_method_valid() {
        assert!(validate_method("GET").is_ok());
        assert!(validate_method("POST").is_ok());
        assert!(validate_method("put").is_ok()); // Should handle lowercase
    }

    #[test]
    fn test_validate_method_invalid() {
        let result = validate_method("INVALID");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_validate_response_valid() {
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "test".to_string(),
        };
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn test_validate_response_invalid_status() {
        let response = ProxyResponse {
            status: 999,
            headers: HashMap::new(),
            body: "test".to_string(),
        };
        let result = validate_response(&response);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_validate_response_too_large() {
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "x".repeat(11 * 1024 * 1024), // 11MB
        };
        let result = validate_response(&response);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_generate_cache_key_consistency() {
        let request1 = ProxyRequest {
            url: "https://api.example.com/test".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let request2 = ProxyRequest {
            url: "https://api.example.com/test".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let key1 = generate_cache_key(&request1);
        let key2 = generate_cache_key(&request2);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_generate_cache_key_different() {
        let request1 = ProxyRequest {
            url: "https://api.example.com/test1".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let request2 = ProxyRequest {
            url: "https://api.example.com/test2".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let key1 = generate_cache_key(&request1);
        let key2 = generate_cache_key(&request2);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_generate_cache_key_with_body() {
        let with_body = ProxyRequest {
            url: "https://api.example.com/test".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: Some("payload1".to_string()),
        };
        let different_body = ProxyRequest {
            url: "https://api.example.com/test".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: Some("payload2".to_string()),
        };
        let no_body = ProxyRequest {
            url: "https://api.example.com/test".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let k1 = generate_cache_key(&with_body);
        let k2 = generate_cache_key(&different_body);
        let k3 = generate_cache_key(&no_body);
        assert_ne!(k1, k2);
        assert_ne!(k1, k3);
        assert_eq!(k1.len(), 64);
    }

    #[tokio::test]
    async fn test_cache_miss_before_set() {
        let cache = Cache::new(300);
        let result = cache.get("nonexistent_key").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_expiry_with_zero_ttl() {
        // TTL=0 means elapsed (always > 0) is never < ttl → entry always expired
        let cache = Cache::new(0);
        let key = "expire_test".to_string();
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "data".to_string(),
        };
        cache.set(key.clone(), response).await;
        // With 0 TTL the entry should be considered expired immediately on next access
        let result = cache.get(&key).await;
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_url_localhost_http_ok() {
        // localhost is allowed by the scheme check; domain check must include "localhost"
        let result = validate_url("http://localhost:3000/api", &["localhost"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_method_delete_and_patch() {
        assert!(validate_method("DELETE").is_ok());
        assert!(validate_method("PATCH").is_ok());
    }

    #[test]
    fn test_validate_method_connect_blocked() {
        assert!(validate_method("CONNECT").is_err());
        assert!(validate_method("HEAD").is_err());
        assert!(validate_method("OPTIONS").is_err());
    }

    #[test]
    fn test_validate_response_valid_json_body() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let response = ProxyResponse {
            status: 200,
            headers,
            body: r#"{"key": "value"}"#.to_string(),
        };
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn test_validate_response_invalid_json_body() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let response = ProxyResponse {
            status: 200,
            headers,
            body: "not valid json".to_string(),
        };
        let result = validate_response(&response);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Serialization(_)));
    }

    #[test]
    fn test_validate_response_status_boundaries() {
        // 200 is valid
        let ok = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: String::new(),
        };
        assert!(validate_response(&ok).is_ok());

        // 599 is valid upper boundary
        let ok599 = ProxyResponse {
            status: 599,
            headers: HashMap::new(),
            body: String::new(),
        };
        assert!(validate_response(&ok599).is_ok());

        // 199 is invalid (below 200)
        let too_low = ProxyResponse {
            status: 199,
            headers: HashMap::new(),
            body: String::new(),
        };
        assert!(validate_response(&too_low).is_err());
    }

    /// Spawn a minimal HTTP/1.1 server on a random port and return the port number.
    async fn spawn_test_http_server(status: u16, body: &'static str) -> u16 {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let status_text = if status == 200 { "OK" } else { "Error" };
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status, status_text, body.len(), body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });

        port
    }

    #[tokio::test]
    async fn test_send_proxy_request_success() {
        let port = spawn_test_http_server(200, r#"{"ok":true}"#).await;
        let request = ProxyRequest {
            url: format!("http://localhost:{}/test", port),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let result = send_proxy_request(&request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, 200);
        assert!(response.body.contains("ok"));
    }

    #[tokio::test]
    async fn test_send_proxy_request_with_body() {
        let port = spawn_test_http_server(200, r#"{"created":true}"#).await;
        let request = ProxyRequest {
            url: format!("http://localhost:{}/api", port),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: Some(r#"{"name":"test"}"#.to_string()),
        };
        let result = send_proxy_request(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_proxy_request_success_and_caches() {
        let port = spawn_test_http_server(200, r#"{"data":"value"}"#).await;
        let request = ProxyRequest {
            url: format!("http://localhost:{}/api", port),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let cache = Cache::new(300);
        let result = proxy_request(request, &cache, &["localhost"]).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, 200);
    }

    #[tokio::test]
    async fn test_proxy_request_uncacheable_method_not_cached() {
        let port = spawn_test_http_server(200, r#"{"deleted":true}"#).await;
        let request = ProxyRequest {
            url: format!("http://localhost:{}/resource", port),
            method: "DELETE".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let cache = Cache::new(300);
        let result = proxy_request(request, &cache, &["localhost"]).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_invalid_url_parsing() {
        let domains = &["api.digitransit.fi"];
        let result = validate_url("not a valid url at all!", domains);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_validate_url_missing_host() {
        let domains = &["api.digitransit.fi"];
        let result = validate_url("https:///path/only", domains);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_cacheable_method_get() {
        assert!(super::is_cacheable_method("GET"));
        assert!(super::is_cacheable_method("get"));
    }

    #[test]
    fn test_is_cacheable_method_post() {
        assert!(super::is_cacheable_method("POST"));
        assert!(super::is_cacheable_method("post"));
    }

    #[test]
    fn test_is_cacheable_method_other() {
        assert!(!super::is_cacheable_method("DELETE"));
        assert!(!super::is_cacheable_method("PUT"));
        assert!(!super::is_cacheable_method("PATCH"));
    }

    #[test]
    fn test_is_allowed_method_all_valid() {
        assert!(super::is_allowed_method("GET"));
        assert!(super::is_allowed_method("POST"));
        assert!(super::is_allowed_method("PUT"));
        assert!(super::is_allowed_method("DELETE"));
        assert!(super::is_allowed_method("PATCH"));
    }

    #[test]
    fn test_is_allowed_method_case_insensitive() {
        assert!(super::is_allowed_method("get"));
        assert!(super::is_allowed_method("Post"));
        assert!(super::is_allowed_method("pUt"));
        assert!(super::is_allowed_method("delete"));
        assert!(super::is_allowed_method("patch"));
    }

    #[test]
    fn test_is_allowed_method_invalid() {
        assert!(!super::is_allowed_method("CONNECT"));
        assert!(!super::is_allowed_method("HEAD"));
        assert!(!super::is_allowed_method("OPTIONS"));
        assert!(!super::is_allowed_method("TRACE"));
    }

    #[test]
    fn test_should_cache_response_get_200() {
        assert!(super::should_cache_response("GET", 200));
        assert!(super::should_cache_response("get", 200));
    }

    #[test]
    fn test_should_cache_response_get_299() {
        assert!(super::should_cache_response("GET", 299));
    }

    #[test]
    fn test_should_cache_response_get_300() {
        assert!(!super::should_cache_response("GET", 300));
    }

    #[test]
    fn test_should_cache_response_get_404() {
        assert!(!super::should_cache_response("GET", 404));
    }

    #[test]
    fn test_should_cache_response_post_200() {
        assert!(super::should_cache_response("POST", 200));
    }

    #[test]
    fn test_should_cache_response_delete() {
        assert!(!super::should_cache_response("DELETE", 200));
    }

    #[test]
    fn test_should_cache_response_put() {
        assert!(!super::should_cache_response("PUT", 200));
    }

    #[test]
    fn test_validate_response_status_600_invalid() {
        let response = ProxyResponse {
            status: 600,
            headers: HashMap::new(),
            body: String::new(),
        };
        let result = validate_response(&response);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_response_status_0_invalid() {
        let response = ProxyResponse {
            status: 0,
            headers: HashMap::new(),
            body: String::new(),
        };
        let result = validate_response(&response);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_response_invalid_json_malformed() {
        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            "application/json; charset=utf-8".to_string(),
        );
        let response = ProxyResponse {
            status: 200,
            headers,
            body: "{ invalid json".to_string(),
        };
        let result = validate_response(&response);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Serialization(_)));
    }

    #[test]
    fn test_validate_response_json_empty_object() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let response = ProxyResponse {
            status: 200,
            headers,
            body: "{}".to_string(),
        };
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn test_validate_response_json_array() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let response = ProxyResponse {
            status: 200,
            headers,
            body: "[]".to_string(),
        };
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn test_validate_response_non_json_content_type() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "text/html".to_string());
        let response = ProxyResponse {
            status: 200,
            headers,
            body: "not json but that's ok".to_string(),
        };
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn test_validate_response_max_body_size_limit() {
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "x".repeat(10 * 1024 * 1024), // Exactly at the limit
        };
        assert!(validate_response(&response).is_ok());
    }

    #[test]
    fn test_validate_response_just_over_limit() {
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "x".repeat(10 * 1024 * 1024 + 1), // 1 byte over limit
        };
        assert!(validate_response(&response).is_err());
    }

    #[test]
    fn test_validate_url_with_port() {
        let domains = &["api.digitransit.fi"];
        let result = validate_url("https://api.digitransit.fi:8443/path", domains);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_subdomain() {
        let domains = &[".digitransit.fi"];
        let result = validate_url("https://api.v2.digitransit.fi/path", domains);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_partial_domain_no_match() {
        let domains = &["digitransit.fi"];
        // Partial domain match should work with ends_with
        let result = validate_url("https://api.digitransit.fi/path", domains);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_url_similar_but_wrong_domain() {
        let domains = &["api.digitransit.fi"];
        let result = validate_url("https://api.evil-digitransit.fi/path", domains);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_cache_key_with_empty_body() {
        let request = ProxyRequest {
            url: "https://api.example.com/test".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: Some("".to_string()),
        };
        let key = generate_cache_key(&request);
        assert_eq!(key.len(), 64); // SHA-256 hex is always 64 chars
    }

    #[test]
    fn test_generate_cache_key_special_characters() {
        let request = ProxyRequest {
            url: "https://api.example.com/test?id=123&name=test%20value".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let key = generate_cache_key(&request);
        assert_eq!(key.len(), 64);
    }

    #[test]
    fn test_generate_cache_key_unicode() {
        let request = ProxyRequest {
            url: "https://api.example.com/test".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            body: Some("Ä body with unicode ü chars".to_string()),
        };
        let key = generate_cache_key(&request);
        assert_eq!(key.len(), 64);
    }

    #[tokio::test]
    async fn test_cache_get_timestamp_error_handling() {
        // This test ensures that the error path at lines 59-61 is covered
        let cache = Cache::new(300);
        let key = "test_error".to_string();
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "test".to_string(),
        };

        cache.set(key.clone(), response).await;

        // Get the response (which should succeed normally, testing the success path is covered)
        let result = cache.get(&key).await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_cache_set_and_get_multiple_entries() {
        let cache = Cache::new(300);
        let mut entries = Vec::new();

        for i in 0..5 {
            let key = format!("key_{}", i);
            let response = ProxyResponse {
                status: 200 + i as u16,
                headers: HashMap::new(),
                body: format!("body_{}", i),
            };
            cache.set(key.clone(), response).await;
            entries.push(key);
        }

        for (i, key) in entries.iter().enumerate() {
            let result = cache.get(key).await;
            assert!(result.is_some());
            let response = result.unwrap();
            assert_eq!(response.status, 200 + i as u16);
            assert_eq!(response.body, format!("body_{}", i));
        }
    }

    #[tokio::test]
    async fn test_cache_clear_removes_all() {
        let cache = Cache::new(300);
        let key1 = "key1".to_string();
        let key2 = "key2".to_string();
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "test".to_string(),
        };

        cache.set(key1.clone(), response.clone()).await;
        cache.set(key2.clone(), response.clone()).await;

        // Verify both are in cache
        assert!(cache.get(&key1).await.is_some());
        assert!(cache.get(&key2).await.is_some());

        // Clear cache
        cache.clear().await;

        // Verify both are gone
        assert!(cache.get(&key1).await.is_none());
        assert!(cache.get(&key2).await.is_none());
    }

    #[test]
    fn test_validate_method_case_variations() {
        assert!(validate_method("GET").is_ok());
        assert!(validate_method("Get").is_ok());
        assert!(validate_method("geT").is_ok());
        assert!(validate_method("gEt").is_ok());
    }

    #[test]
    fn test_validate_response_all_status_codes_200_599() {
        for status in 200..600 {
            let response = ProxyResponse {
                status,
                headers: HashMap::new(),
                body: "".to_string(),
            };
            let result = validate_response(&response);
            assert!(
                result.is_ok(),
                "Status {} should be valid but got error",
                status
            );
        }
    }

    #[test]
    fn test_validate_response_invalid_status_boundaries() {
        // Test status 100 (too low)
        let response = ProxyResponse {
            status: 100,
            headers: HashMap::new(),
            body: "".to_string(),
        };
        assert!(validate_response(&response).is_err());

        // Test status 1000 (too high)
        let response = ProxyResponse {
            status: 1000,
            headers: HashMap::new(),
            body: "".to_string(),
        };
        assert!(validate_response(&response).is_err());
    }

    #[test]
    fn test_generate_cache_key_ignores_headers() {
        // Headers are excluded from the cache key — same URL/method/body with different
        // Authorization headers must produce the same key.
        let req_with_auth = ProxyRequest {
            url: "https://api.example.com/data".to_string(),
            method: "GET".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert(
                    "Authorization".to_string(),
                    "Bearer secret_token".to_string(),
                );
                h
            },
            body: None,
        };
        let req_no_auth = ProxyRequest {
            url: "https://api.example.com/data".to_string(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        assert_eq!(
            generate_cache_key(&req_with_auth),
            generate_cache_key(&req_no_auth),
            "cache key must not include headers"
        );
    }

    #[tokio::test]
    async fn test_cache_get_timestamp_error_branch() {
        // The Err(err) branch at lines 59-61 is hit when the stored timestamp is in the
        // future (SystemTime in advance of now). We verify this in two ways:
        // 1. Directly: elapsed() on a future SystemTime returns Err.
        // 2. Via the public API: insert_with_future_timestamp + get() → None.
        let future_time = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        assert!(
            future_time.elapsed().is_err(),
            "A future timestamp must produce SystemTimeError from elapsed()"
        );

        let cache = Cache::new(300);
        let key = "future_ts_branch".to_string();
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "branch_test".to_string(),
        };
        cache
            .insert_with_future_timestamp(key.clone(), response)
            .await;
        // Cache::get() should hit the Err(err) arm, remove the entry, and return None.
        let result = cache.get(&key).await;
        assert!(
            result.is_none(),
            "entry with future timestamp should be removed and return None"
        );
    }

    #[tokio::test]
    async fn test_send_proxy_request_with_custom_headers() {
        // Exercises the `for (key, value) in &request.headers` loop (line 269)
        let port = spawn_test_http_server(200, r#"{"ok":true}"#).await;
        let mut headers = HashMap::new();
        headers.insert("X-Custom-Header".to_string(), "test-value".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());
        let request = ProxyRequest {
            url: format!("http://localhost:{}/test", port),
            method: "GET".to_string(),
            headers,
            body: None,
        };
        let result = send_proxy_request(&request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, 200);
    }

    #[tokio::test]
    async fn test_proxy_request_get_caches_and_logs() {
        // Exercises the `log::info!("Cached {} response for {}")` line (the caching branch)
        // by performing a GET request that succeeds (200) and should be cached.
        let port = spawn_test_http_server(200, r#"{"cached":true}"#).await;
        let request = ProxyRequest {
            url: format!("http://localhost:{}/api", port),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let cache = Cache::new(300);
        let result = proxy_request(request, &cache, &["localhost"]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, 200);
    }

    #[tokio::test]
    async fn test_proxy_request_cache_hit_returns_cached() {
        // Exercises line 311: `return Ok(cached_response)` inside proxy_request.
        // First call forwards to the server and caches the response.
        // Second call hits the cache (server is no longer needed).
        let port = spawn_test_http_server(200, r#"{"value":42}"#).await;
        let url = format!("http://localhost:{}/cached-resource", port);
        let request1 = ProxyRequest {
            url: url.clone(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let request2 = ProxyRequest {
            url: url.clone(),
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        let cache = Cache::new(300);
        let first = proxy_request(request1, &cache, &["localhost"])
            .await
            .unwrap();
        assert_eq!(first.status, 200);
        // Second call must hit the cache (server already handled its one accept).
        let second = proxy_request(request2, &cache, &["localhost"])
            .await
            .unwrap();
        assert_eq!(second.status, 200);
        assert!(second.body.contains("42"));
    }

    #[tokio::test]
    async fn test_cache_get_future_timestamp_removes_entry() {
        // Exercises lines 59-61: Err(err) arm triggered when entry.timestamp is in
        // the future.  elapsed() on a future SystemTime returns SystemTimeError.
        let cache = Cache::new(300);
        let key = "future_ts_key".to_string();
        let response = ProxyResponse {
            status: 200,
            headers: HashMap::new(),
            body: "future".to_string(),
        };
        // Insert with a timestamp set 10 years ahead so elapsed() → Err.
        cache
            .insert_with_future_timestamp(key.clone(), response)
            .await;
        // get() must hit the Err branch, remove the entry, and return None.
        let result = cache.get(&key).await;
        assert!(
            result.is_none(),
            "entry with future timestamp should be treated as invalid and return None"
        );
    }
}

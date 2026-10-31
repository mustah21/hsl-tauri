use serde::Serialize;
use std::fmt;

/// Custom error type for the application
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "message")]
pub enum AppError {
    /// File system I/O errors
    Io(String),
    /// Sled database errors
    Database(String),
    /// Serialization/deserialization errors (bitcode, JSON, etc.)
    Serialization(String),
    /// HTTP request errors
    Http(String),
    /// Input validation errors
    Validation(String),
    /// System time errors
    Time(String),
    /// Cache operation errors
    Cache(String),
    /// Resource not found
    NotFound(String),
    /// HTTP response builder errors
    ResponseBuilder(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(msg) => write!(f, "I/O error: {}", msg),
            AppError::Database(msg) => write!(f, "Database error: {}", msg),
            AppError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            AppError::Http(msg) => write!(f, "HTTP error: {}", msg),
            AppError::Validation(msg) => write!(f, "Validation error: {}", msg),
            AppError::Time(msg) => write!(f, "Time error: {}", msg),
            AppError::Cache(msg) => write!(f, "Cache error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::ResponseBuilder(msg) => write!(f, "Response builder error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

// Implement From conversions for common error types
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

impl From<sled::Error> for AppError {
    fn from(err: sled::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::Http(err.to_string())
    }
}

impl From<url::ParseError> for AppError {
    fn from(err: url::ParseError) -> Self {
        AppError::Validation(err.to_string())
    }
}

impl From<std::time::SystemTimeError> for AppError {
    fn from(err: std::time::SystemTimeError) -> Self {
        AppError::Time(err.to_string())
    }
}

impl From<bitcode::Error> for AppError {
    fn from(err: bitcode::Error) -> Self {
        AppError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_display_io_error() {
        let err = AppError::Io("file not found".to_string());
        assert_eq!(format!("{}", err), "I/O error: file not found");
    }

    #[test]
    fn test_display_validation_error() {
        let err = AppError::Validation("invalid URL".to_string());
        assert_eq!(format!("{}", err), "Validation error: invalid URL");
    }

    #[test]
    fn test_display_not_found_error() {
        let err = AppError::NotFound("resource missing".to_string());
        assert_eq!(format!("{}", err), "Not found: resource missing");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "test error");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
        assert!(format!("{}", app_err).contains("test error"));
    }

    #[test]
    fn test_from_url_parse_error() {
        let url_err = url::Url::parse("not a url").unwrap_err();
        let app_err: AppError = url_err.into();
        assert!(matches!(app_err, AppError::Validation(_)));
    }

    #[test]
    fn test_error_is_serializable() {
        let err = AppError::Cache("cache full".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("Cache"));
        assert!(json.contains("cache full"));
    }

    #[test]
    fn test_display_database_error() {
        let err = AppError::Database("connection refused".to_string());
        assert_eq!(format!("{}", err), "Database error: connection refused");
    }

    #[test]
    fn test_display_serialization_error() {
        let err = AppError::Serialization("bad bytes".to_string());
        assert_eq!(format!("{}", err), "Serialization error: bad bytes");
    }

    #[test]
    fn test_display_http_error() {
        let err = AppError::Http("503 unavailable".to_string());
        assert_eq!(format!("{}", err), "HTTP error: 503 unavailable");
    }

    #[test]
    fn test_display_time_error() {
        let err = AppError::Time("clock skew".to_string());
        assert_eq!(format!("{}", err), "Time error: clock skew");
    }

    #[test]
    fn test_display_response_builder_error() {
        let err = AppError::ResponseBuilder("header rejected".to_string());
        assert_eq!(
            format!("{}", err),
            "Response builder error: header rejected"
        );
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let app_err: AppError = json_err.into();
        assert!(matches!(app_err, AppError::Serialization(_)));
        assert!(format!("{}", app_err).starts_with("Serialization error:"));
    }

    #[test]
    fn test_from_sled_error() {
        // A file path where sled cannot create a directory triggers a Database error
        let tmp_file = std::env::temp_dir().join("sled_error_test_marker.txt");
        let _ = io::Write::write_all(
            &mut std::fs::File::create(&tmp_file).unwrap(),
            b"not a database",
        );
        let impossible_path = tmp_file.join("subdir");
        if let Err(sled_err) = sled::open(&impossible_path) {
            let app_err: AppError = sled_err.into();
            assert!(matches!(app_err, AppError::Database(_)));
            assert!(format!("{}", app_err).starts_with("Database error:"));
        }
    }

    #[test]
    fn test_from_bitcode_error() {
        // Random bytes that cannot be deserialized as a String
        let bad: &[u8] = &[0xFF, 0xFE, 0xFD, 0x01, 0x02, 0x03];
        if let Err(bc_err) = bitcode::deserialize::<String>(bad) {
            let app_err: AppError = bc_err.into();
            assert!(matches!(app_err, AppError::Serialization(_)));
            assert!(format!("{}", app_err).starts_with("Serialization error:"));
        }
    }

    #[test]
    fn test_display_cache_error() {
        let err = AppError::Cache("storage full".to_string());
        assert_eq!(format!("{}", err), "Cache error: storage full");
    }

    #[tokio::test]
    async fn test_from_reqwest_error() {
        // An invalid URL scheme causes reqwest to return an error without network access
        let result = reqwest::Client::new().get("::not_a_url").send().await;
        assert!(result.is_err());
        let app_err = AppError::from(result.unwrap_err());
        assert!(matches!(app_err, AppError::Http(_)));
    }

    #[test]
    fn test_from_system_time_error() {
        // UNIX_EPOCH.duration_since(now) produces a SystemTimeError because now > UNIX_EPOCH
        let err = std::time::SystemTime::UNIX_EPOCH
            .duration_since(std::time::SystemTime::now())
            .unwrap_err();
        let app_err = AppError::from(err);
        assert!(matches!(app_err, AppError::Time(_)));
    }
}

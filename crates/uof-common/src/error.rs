use thiserror::Error;

pub type Result<T> = std::result::Result<T, UofError>;

#[derive(Debug, Error)]
pub enum UofError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for UofError {
    fn from(e: anyhow::Error) -> Self {
        UofError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_error_display_config() {
        let err = UofError::Config("missing field".to_string());
        assert_eq!(err.to_string(), "configuration error: missing field");
    }

    #[test]
    fn test_error_display_internal() {
        let err = UofError::Internal("something went wrong".to_string());
        assert_eq!(err.to_string(), "internal error: something went wrong");
    }

    #[test]
    fn test_error_source() {
        let err = UofError::Internal("test".to_string());
        // UofError itself has no source
        assert!(err.source().is_none());
    }

    #[test]
    fn test_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("anyhow error");
        let uof_err: UofError = anyhow_err.into();

        match uof_err {
            UofError::Internal(msg) => {
                assert!(msg.contains("anyhow error"));
            }
            _ => panic!("Expected Internal variant"),
        }
    }

    #[test]
    fn test_error_debug() {
        let err = UofError::Config("debug test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("debug test"));
    }

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(UofError::Config("failed".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed"));
    }
}


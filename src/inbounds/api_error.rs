use crate::domain::services::orderbook_manager::OrderbookManagerError;
use thiserror::Error;

/// +----------------------------------------------------------+
/// | STRUCTS | TRAITS | ENUMS | FUNCTIONS                     |
/// +----------+-------+-------+------------------------------+
/// | Enums:                                                   |
/// |   - ApiError                                             |
/// +----------------------------------------------------------+

/// Represents errors that can occur in the API layer.
#[derive(Debug, Error)]
pub enum ApiError {
    /// The request was malformed or invalid.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// An internal server error occurred.
    #[error("Internal server error: {0}")]
    InternalError(#[from] anyhow::Error),
}

impl From<OrderbookManagerError> for ApiError {
    fn from(err: OrderbookManagerError) -> Self {
        ApiError::InternalError(anyhow::anyhow!("{}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_display() {
        let bad_request = ApiError::BadRequest("Invalid parameter".to_string());
        assert_eq!(
            format!("{}", bad_request),
            "Bad request: Invalid parameter"
        );

        let internal_error = ApiError::InternalError(anyhow::anyhow!("Database error"));
        assert_eq!(
            format!("{}", internal_error),
            "Internal server error: Database error"
        );
    }
} 
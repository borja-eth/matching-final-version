//--------------------------------------------------------------------------------------------------
// ENUMS
//--------------------------------------------------------------------------------------------------
// | Name            | Description                                      | Key Methods         |
// |-----------------|--------------------------------------------------|---------------------|
// | ApiError        | Error types for the API                          | from                |
//--------------------------------------------------------------------------------------------------

use axum::{
    response::{Response, IntoResponse},
    http::StatusCode,
    Json,
};
use serde_json::json;
use thiserror::Error;

use crate::matching_engine::MatchingError;

/// Type alias for Result with ApiError
pub type ApiResult<T> = Result<T, ApiError>;

/// API-specific error types
#[derive(Error, Debug, Clone)]
pub enum ApiError {
    /// The requested resource was not found
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    /// The request was invalid
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    /// Internal server error
    #[error("Internal server error: {0}")]
    Internal(String),
    
    /// Matching engine specific error
    #[error("Matching engine error: {0}")]
    MatchingEngine(String),
    
    /// The request contains invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    
    /// The request is valid but cannot be processed
    #[error("Unprocessable entity: {0}")]
    Unprocessable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::MatchingEngine(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::InvalidParams(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Unprocessable(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
        };
        
        let body = Json(json!({
            "error": {
                "message": error_message,
                "code": status.as_u16()
            }
        }));
        
        (status, body).into_response()
    }
}

impl From<MatchingError> for ApiError {
    fn from(err: MatchingError) -> Self {
        match err {
            MatchingError::InvalidOrder(msg) => Self::BadRequest(msg),
            MatchingError::OrderNotFound(id) => Self::NotFound(format!("Order {} not found", id)),
            MatchingError::InsufficientLiquidity => Self::Unprocessable("Insufficient liquidity".to_string()),
            MatchingError::WrongInstrument => Self::BadRequest("Wrong instrument".to_string()),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        Self::BadRequest(format!("JSON error: {}", err))
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(format!("IO error: {}", err))
    }
} 
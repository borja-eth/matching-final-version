use thiserror::Error;
use uuid::Uuid;

use crate::domain::models::types::OrderStatus;

pub mod matching_engine;

/// Re-export key types for convenience
pub use self::matching_engine::{MatchingEngine, MatchResult};

/// Errors that can occur during matching engine operations.
#[derive(Debug, Error)]
pub enum MatchingError {
    /// Order not found in any orderbook
    #[error("Order {0} not found")]
    OrderNotFound(Uuid),

    /// Invalid instrument ID
    #[error("Invalid instrument ID: {0}")]
    InvalidInstrument(Uuid),

    /// Invalid order state
    #[error("Order {id} is in an invalid state: {status:?}")]
    InvalidOrderState {
        id: Uuid,
        status: OrderStatus,
    },

    /// Orderbook error occurred
    #[error("Orderbook error: {0}")]
    OrderbookError(#[from] crate::domain::services::orderbook::OrderbookError),

    /// Market order with no opposing orders
    #[error("Market order has no opposing orders to match against")]
    NoOpposingOrders,

    /// Generic matching engine error
    #[error("Matching engine error: {0}")]
    Other(String),
}


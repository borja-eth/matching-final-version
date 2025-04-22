use thiserror::Error;
use uuid::Uuid;

pub mod orderbook;
pub mod depth;

/// Errors that can occur within the orderbook service.
///
/// This enum represents the various error conditions that can arise
/// during orderbook operations such as adding, removing, or querying orders.
#[derive(Debug, Error)]
pub enum OrderbookError {
    /// Internal orderbook error
    #[error("Internal orderbook error: {0}")]
    Internal(String),
    
    /// Order not found in the orderbook
    #[error("Order {0} not found in the orderbook")]
    OrderNotFound(Uuid),
    
    /// Order is for a different instrument than this orderbook
    #[error("Order is for wrong instrument (expected {expected}, got {got})")]
    WrongInstrument {
        expected: Uuid,
        got: Uuid,
    },

    /// Market orders cannot be added to the book
    #[error("Market orders cannot be added to the orderbook (no limit price)")]
    NoLimitPrice,

    /// Invalid price level
    #[error("Invalid price level: {0}")]
    InvalidPrice(i64),

    /// Invalid order quantity
    #[error("Invalid order quantity: {0}")]
    InvalidQuantity(u64),
    
    /// Generic orderbook error
    #[error("Orderbook error: {0}")]
    Other(String),
}

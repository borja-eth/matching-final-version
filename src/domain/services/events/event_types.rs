//--------------------------------------------------------------------------------------------------
// STRUCTS & ENUMS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | MatchingEngineEvent     | Event variants for the matching engine           | clone, send, sync |
// | EventError              | Error types for event processing                 | error, from       |
//--------------------------------------------------------------------------------------------------

use chrono::Utc;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use uuid::Uuid;
use crate::domain::models::types::{Order, Trade, OrderStatus};
use crate::domain::services::orderbook::depth::DepthSnapshot;

/// Errors that can occur in the event system
#[derive(Error, Debug, Clone)]
pub enum EventError {
    /// Failed to publish an event (e.g., no subscribers or channel full)
    #[error("Failed to publish event: {0}")]
    PublishError(String),
    
    /// Failed to process an event
    #[error("Failed to process event: {0}")]
    ProcessingError(String),
    
    /// Event handler not found for event type
    #[error("No handler registered for event type: {0}")]
    HandlerNotFound(String),
}

/// Type alias for Result with EventError
pub type EventResult<T> = Result<T, EventError>;

/// Represents events that can occur in the matching engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchingEngineEvent {
    /// Generated when an order is added to the book
    OrderAdded {
        /// The order that was added
        order: Order,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when an order is matched (partially or fully)
    OrderMatched {
        /// The order that was matched
        order: Order,
        /// Amount of the order that was matched
        matched_quantity: u64,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when an order is cancelled
    OrderCancelled {
        /// The order that was cancelled
        order: Order,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when an order's status changes
    OrderStatusChanged {
        /// The order ID
        order_id: Uuid,
        /// Previous status
        previous_status: OrderStatus,
        /// New status
        new_status: OrderStatus,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when a trade is executed
    TradeExecuted {
        /// The trade that was executed
        trade: Trade,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when the depth is updated
    DepthUpdated {
        /// The updated depth snapshot
        depth: DepthSnapshot,
        /// Timestamp when the event occurred 
        timestamp: chrono::DateTime<Utc>,
    },
} 
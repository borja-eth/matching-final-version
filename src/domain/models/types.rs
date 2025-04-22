//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module defines the core data types used throughout the matching engine,
// including orders, trades, and various status/type enums.
//
// | Section            | Description                                                      |
// |--------------------|------------------------------------------------------------------|
// | ENUMS              | Defines discrete sets of values (Side, OrderType, OrderStatus...). |
// | STRUCTS            | Defines the structure of Orders and Trades.                      |
// | Potential Errors   | Defines errors related to type handling.                         |
// | TESTS              | Contains unit tests for the defined types.                       |
//--------------------------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------
//  ENUMS
//--------------------------------------------------------------------------------------------------
// | Name          | Description                               |
// |---------------|-------------------------------------------|
// | Side          | Represents the side of an order (Buy/Sell). |
// | OrderType     | Represents the type of an order.          |
// | OrderStatus   | Represents the status of an order.        |
// | TriggerType   | How a trigger price is evaluated.         |
// | CreatedFrom   | Source of order creation.                 |
//--------------------------------------------------------------------------------------------------
use chrono::{DateTime, Utc};
use thiserror::Error; // Added early for consistency, though errors defined later
use uuid::Uuid;
use serde::{Serialize, Deserialize};


/// Represents the side of an order (Buy or Sell).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    #[serde(rename_all = "UPPERCASE")]
    /// A buy order (also called buy).
    Bid,
    /// A sell order (also called sell).
    Ask,
}

#[allow(dead_code)]
impl Side {
    pub fn opposite(&self) -> Self {
        match self {
            Self::Bid => Self::Ask,
            Self::Ask => Self::Bid,
        }
    }
}

/// Represents the type of an order, influencing its matching behavior.
/// Maps to order types defined in `@roxom.md`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename_all = "lowercase")]
    /// An order that executes at a specific price or better.
    Limit,
    /// An order that executes immediately at the best available market price.
    Market,
    /// A conditional order that becomes a Market order when the trigger price is reached.
    Stop,
    /// A conditional order that becomes a Limit order when the trigger price is reached.
    StopLimit,
    // Liquidation, // Consider if needed for core matching logic initially
    // Adl,         // Consider if needed for core matching logic initially
}

/// Defines how long an order remains active in the order book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeInForce {
        #[serde(rename_all = "lowercase")]

    /// Good Till Cancel - remains active until explicitly cancelled
    GTC,
    /// Immediate Or Cancel - must be filled immediately (fully or partially) or cancelled
    IOC,
}

/// Represents the lifecycle status of an order within the matching engine.
/// Maps to statuses defined in `@roxom.md`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderStatus {
    /// The order has been acknowledged by the engine.
    Submitted,
    /// The order has been accepted by the engine but not yet matched or filled.
    Unfilled,
    /// The order has been partially filled.
    PartiallyFilled,
    /// The order was partially filled and then cancelled.
    PartiallyFilledCancelled,
    /// The order has been completely filled.
    Filled,
    /// A conditional order (e.g., Stop) that is waiting for its trigger condition.
    WaitingTrigger,
    /// The order was cancelled before being fully filled.
    Cancelled,
    /// The order was rejected by the engine.
    Rejected,
    // Agregar Self Trade Prevention Cancelled

}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::GTC
    }
}

/// Specifies the price type used to evaluate the trigger condition for conditional orders.
/// Defined in `@roxom.md`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerType {
    /// Trigger is evaluated against the last traded price.
    LastPrice,
    // Add other types like MarkPrice, IndexPrice if needed later
}

/// Indicates the origin system or interface that created the order.
/// Defined in `@roxom.md`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreatedFrom {
    /// Order created via an API client.
    Api,
    /// Order created via a user interface/frontend.
    Front,
    // Add other sources like 'System' for liquidations, ADL, etc. if needed
}


//--------------------------------------------------------------------------------------------------
//  STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name          | Description                                   |
// |---------------|-----------------------------------------------|
// | Order         | Represents a trading order in the system.     |
// | Trade         | Represents a completed trade between orders.  |
//--------------------------------------------------------------------------------------------------

/// Represents a trading order, based on `@roxom.md`.
/// Uses Decimal for price/quantity precision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Order {
    /// Unique identifier for the order (internal).
    pub id: Uuid,
    /// Optional external identifier provided by the client.
    pub ext_id: Option<String>,
    /// Identifier for the account placing the order.
    pub account_id: Uuid,
    /// Side of the order (Buy or Sell).
    pub side: Side,
    /// Type of the order (Limit, Market, etc.).
    pub order_type: OrderType,
    /// Limit price for Limit/StopLimit orders. Stored as Decimal.
    pub limit_price: Option<i64>, // Use Option for Market orders
    /// Initial order quantity in base units. Stored as Decimal.
    pub base_amount: u64,
    /// Remaining quantity available to trade in base units. Stored as u64.
    pub remaining_base: u64,
    /// Time in force policy for the order (GTC, IOC, FOK, etc.)
    pub time_in_force: TimeInForce,
    /// Timestamp of order creation.
    pub created_at: DateTime<Utc>,
    /// Current status of the order.
    pub status: OrderStatus,
    /// Identifier for the instrument being traded.
    pub instrument_id: Uuid,
    /// Trigger price for Stop/StopLimit orders. Stored as i64.
    pub trigger_price: Option<i64>,
    /// Remaining quantity available to trade in quote units.
    /// Although often calculated (`remaining_base * price`), it's stored here directly
    /// for potential performance or specific model requirements.
    pub remaining_quote: u64,
    /// Quantity filled in quote units. Stored as u64.
    pub filled_quote: u64,
    /// Quantity filled in base units. Stored as u64.
    pub filled_base: u64,
    /// Timestamp for when the order expires (used for GTD orders).
    pub expiration_date: DateTime<Utc>,
    /// Timestamp of the last update to the order.
    pub updated_at: DateTime<Utc>,
    /// How the trigger price is evaluated (if applicable).
    pub trigger_by: Option<TriggerType>,
    /// Source of the order creation.
    pub created_from: CreatedFrom,
    // Engine specific fields (not in roxom.md directly, but needed for matching)
    /// Sequence number assigned by the engine upon acceptance (for time priority).
    pub sequence_id: u64,
}

/// Represents a completed trade resulting from matching two orders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trade {
    /// Unique identifier for the trade.
    pub id: Uuid,
    /// Identifier for the instrument traded.
    pub instrument_id: Uuid,
    /// ID of the order that was resting on the book (maker).
    pub maker_order_id: Uuid,
    /// ID of the order that matched the resting order (taker).
    pub taker_order_id: Uuid,
    /// Quantity traded in base units. Stored as u64.
    pub base_amount: u64,
    /// Quantity traded in quote units. Stored as u64. Calculated as base_amount * price.
    pub quote_amount: u64,
    /// Price at which the trade occurred. Stored as i64.
    pub price: i64,
    /// Timestamp when the trade occurred.
    pub created_at: DateTime<Utc>,
}


//--------------------------------------------------------------------------------------------------
//  Potential Errors (Initial Placeholder)
//--------------------------------------------------------------------------------------------------
/// Represents errors that can occur during type validation or conversion within this module.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TypeError {
    /// Occurs when attempting to create a `Side` from an unrecognized string or value.
    #[error("Invalid side specified: {0}")]
    InvalidSide(String),
    /// Occurs when attempting to create an `OrderType` from an unrecognized string or value.
    #[error("Invalid order type specified: {0}")]
    InvalidOrderType(String),
    /// Occurs when attempting to create an `OrderStatus` from an unrecognized string or value.
    #[error("Invalid order status specified: {0}")]
    InvalidOrderStatus(String),
    
    /// Occurs when attempting to create a `TimeInForce` from an unrecognized string or value.
    #[error("Invalid time in force specified: {0}")]
    InvalidTimeInForce(String),
    
    /// Occurs when attempting to create a `TriggerType` from an unrecognized string or value.
    #[error("Invalid trigger type specified: {0}")]
    InvalidTriggerType(String),
    
    /// Occurs when attempting to create a `CreatedFrom` from an unrecognized string or value.
    #[error("Invalid created from source specified: {0}")]
    InvalidCreatedFrom(String),
    
    /// Occurs when a required price is missing for a specific order type.
    #[error("Missing required price for order type: {0}")]
    MissingRequiredPrice(String),
    
    /// Occurs when an invalid quantity is specified (e.g., zero or negative).
    #[error("Invalid quantity specified: {0}")]
    InvalidQuantity(String),
    
    /// Occurs when an invalid price is specified (e.g., zero or negative for limit orders).
    #[error("Invalid price specified: {0}")]
    InvalidPrice(String),
    
    /// Occurs when an order operation is attempted with an invalid UUID.
    #[error("Invalid UUID format: {0}")]
    InvalidUuid(String),
    // Add more specific type errors as needed
}


//--------------------------------------------------------------------------------------------------
//  TESTS
//--------------------------------------------------------------------------------------------------
// | Name                       | Description                                      |
// |----------------------------|--------------------------------------------------|
// | test_order_creation        | Verify basic Order struct instantiation.          |
// | test_trade_creation        | Verify basic Trade struct instantiation.          |
// | test_enum_derives          | Check basic enum functionality (clone, copy, eq).|
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
 // For easy Decimal literal creation

    #[test]
    fn test_order_creation() {
        let now = Utc::now();
        let order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("client-order-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Limit,
            instrument_id: Uuid::new_v4(),
            side: Side::Bid,
            limit_price: Some(50000),  // i64 for price
            trigger_price: None,
            base_amount: 100000,       // u64 for quantity (e.g., 1.0 BTC = 100000 satoshis)
            remaining_base: 100000,
            filled_quote: 0,
            filled_base: 0,
            remaining_quote: 5000000000, // 50000 * 100000
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::Submitted,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 1,
            time_in_force: TimeInForce::GTC,
        };
        assert_eq!(order.side, Side::Bid);
        assert_eq!(order.base_amount, 100000);
        assert_eq!(order.status, OrderStatus::Submitted);
        assert_eq!(order.remaining_quote, 5000000000);
    }

    #[test]
    fn test_trade_creation() {
        let now = Utc::now();
        let trade = Trade {
            id: Uuid::new_v4(),
            instrument_id: Uuid::new_v4(),
            maker_order_id: Uuid::new_v4(),
            taker_order_id: Uuid::new_v4(),
            base_amount: 50000,        // 0.5 BTC
            quote_amount: 2500000000,  // 50000 * 50000
            price: 50000,
            created_at: now,
        };
        assert_eq!(trade.base_amount, 50000);
        assert_eq!(trade.price, 50000);
    }

    #[test]
    fn test_enum_derives() {
        // Test Side enum
        let bid_side = Side::Bid;
        let ask_side = Side::Ask;
        assert_ne!(bid_side, ask_side);

        // Test OrderType enum
        let limit = OrderType::Limit;
        let market = OrderType::Market;
        let stop = OrderType::Stop;
        let stop_limit = OrderType::StopLimit;
        assert_ne!(limit, market);
        assert_ne!(stop, stop_limit);

        // Test OrderStatus enum
        let submitted = OrderStatus::Submitted;
        let waiting = OrderStatus::WaitingTrigger;
        let partial = OrderStatus::PartiallyFilled;
        let filled = OrderStatus::Filled;
        let cancelled = OrderStatus::Cancelled;
        let partial_cancelled = OrderStatus::PartiallyFilledCancelled;
        assert_ne!(submitted, filled);
        assert_ne!(partial, cancelled);
        assert_ne!(waiting, partial_cancelled);
    }

    #[test]
    fn test_market_order() {
        let now = Utc::now();
        let market_order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("market-order-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Market,
            instrument_id: Uuid::new_v4(),
            side: Side::Ask,
            limit_price: None,
            trigger_price: None,
            base_amount: 200000,       // 2.0 BTC
            remaining_base: 200000,
            filled_quote: 0,
            filled_base: 0,
            remaining_quote: 0,        // Market orders don't have a price until execution
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::Submitted,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Front,
            sequence_id: 2,
            time_in_force: TimeInForce::IOC,
        };
        assert_eq!(market_order.order_type, OrderType::Market);
        assert_eq!(market_order.side, Side::Ask);
        assert_eq!(market_order.created_from, CreatedFrom::Front);
    }

    #[test]
    fn test_order_status_transitions() {
        let now = Utc::now();
        let mut order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("partial-fill-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Limit,
            instrument_id: Uuid::new_v4(),
            side: Side::Bid,
            limit_price: Some(50000),
            trigger_price: None,
            base_amount: 100000,       // 1.0 BTC
            remaining_base: 50000,     // 0.5 BTC remaining
            filled_quote: 2500000000,  // 25000 * 100000
            filled_base: 50000,        // 0.5 BTC filled
            remaining_quote: 2500000000, // 25000 * 100000 remaining
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::PartiallyFilled,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 4,
            time_in_force: TimeInForce::GTC,
        };

        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.remaining_base, 50000);
        assert_eq!(order.filled_base, 50000);

        // Test transition to Filled
        order.remaining_base = 0;
        order.filled_base = 100000;
        order.filled_quote = 5000000000;
        order.remaining_quote = 0;
        order.status = OrderStatus::Filled;
        assert_eq!(order.status, OrderStatus::Filled);
        assert_eq!(order.remaining_base, 0);
        assert_eq!(order.filled_base, 100000);
    }

    #[test]
    fn test_stop_limit_order() {
        let now = Utc::now();
        let stop_limit_order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("stop-limit-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::StopLimit,
            instrument_id: Uuid::new_v4(),
            side: Side::Ask,
            limit_price: Some(48000),
            trigger_price: Some(47000),
            base_amount: 100000,
            remaining_base: 100000,
            filled_quote: 0,
            filled_base: 0,
            remaining_quote: 4800000000,
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::WaitingTrigger,
            created_at: now,
            updated_at: now,
            trigger_by: Some(TriggerType::LastPrice),
            created_from: CreatedFrom::Api,
            sequence_id: 7,
            time_in_force: TimeInForce::GTC,
        };
        
        assert_eq!(stop_limit_order.order_type, OrderType::StopLimit);
        assert_eq!(stop_limit_order.status, OrderStatus::WaitingTrigger);
        assert_eq!(stop_limit_order.trigger_by, Some(TriggerType::LastPrice));
        assert_eq!(stop_limit_order.limit_price, Some(48000));
        assert_eq!(stop_limit_order.trigger_price, Some(47000));
    }
} 
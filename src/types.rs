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
use rust_decimal::Decimal;
use thiserror::Error; // Added early for consistency, though errors defined later
use uuid::Uuid;


/// Represents the side of an order (Buy or Sell).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// A buy order.
    Bid,
    /// A sell order.
    Ask,
}

/// Represents the type of an order, influencing its matching behavior.
/// Maps to order types defined in `@roxom.md`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderType {
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

/// Represents the lifecycle status of an order within the matching engine.
/// Maps to statuses defined in `@roxom.md`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    // Roxom Gateway Statuses (May not be directly stored/used in matching engine core)
    // PendingNew,
    // PendingCancel,
    // Inactive,
    // Rejected,

    // Engine Core Statuses
    /// The order has been accepted by the engine but not yet matched.
    New,
    /// A conditional order (e.g., Stop) that is waiting for its trigger condition.
    WaitingTrigger,
    /// The order has been partially filled.
    PartiallyFilled,
    /// The order has been completely filled.
    Filled,
    /// The order was cancelled before being fully filled.
    Cancelled,
    /// The order was partially filled and then cancelled.
    PartiallyFilledCancelled,
}

/// Defines how long an order remains active in the order book.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum TimeInForce {
    /// Good Till Cancel - remains active until explicitly cancelled
    GTC,
    /// Immediate Or Cancel - must be filled immediately (fully or partially) or cancelled
    IOC,
}

impl Default for TimeInForce {
    fn default() -> Self {
        Self::GTC
    }
}

/// Specifies the price type used to evaluate the trigger condition for conditional orders.
/// Defined in `@roxom.md`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerType {
    /// Trigger is evaluated against the last traded price.
    LastPrice,
    // Add other types like MarkPrice, IndexPrice if needed later
}

/// Indicates the origin system or interface that created the order.
/// Defined in `@roxom.md`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    /// Unique identifier for the order (internal).
    pub id: Uuid,
    /// Optional external identifier provided by the client.
    pub ext_id: Option<String>,
    /// Identifier for the account placing the order.
    pub account_id: Uuid,
    /// Type of the order (Limit, Market, etc.).
    pub order_type: OrderType,
    /// Identifier for the instrument being traded.
    pub instrument_id: Uuid,
    /// Side of the order (Buy or Sell).
    pub side: Side,
    /// Limit price for Limit/StopLimit orders. Stored as Decimal.
    pub limit_price: Option<Decimal>, // Use Option for Market orders
    /// Trigger price for Stop/StopLimit orders. Stored as Decimal.
    pub trigger_price: Option<Decimal>,
    /// Initial order quantity in base units. Stored as Decimal.
    pub base_amount: Decimal,
    /// Remaining quantity available to trade in quote units.
    /// Although often calculated (`remaining_base * price`), it's stored here directly
    /// for potential performance or specific model requirements.
     pub remaining_quote: Decimal,
    /// Remaining quantity available to trade in base units. Stored as Decimal.
    pub remaining_base: Decimal,
    /// Quantity filled in quote units. Stored as Decimal.
    pub filled_quote: Decimal,
    /// Quantity filled in base units. Stored as Decimal.
    pub filled_base: Decimal,
    /// Timestamp for when the order expires (GTC often represented by a far future date).
    pub expiration_date: DateTime<Utc>,
    /// Current status of the order.
    pub status: OrderStatus,
    /// Timestamp of order creation.
    pub created_at: DateTime<Utc>,
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
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    /// Unique identifier for the trade.
    pub id: Uuid,
    /// Identifier for the instrument traded.
    pub instrument_id: Uuid,
    /// ID of the order that was resting on the book (maker).
    pub maker_order_id: Uuid,
    /// ID of the order that matched the resting order (taker).
    pub taker_order_id: Uuid,
    /// Quantity traded in base units. Stored as Decimal.
    pub base_amount: Decimal,
    /// Quantity traded in quote units. Stored as Decimal. Calculated as base_amount * price.
    pub quote_amount: Decimal, // Renamed from quoteAmountInNanoBTC for clarity w/ Decimal
    /// Price at which the trade occurred. Stored as Decimal.
    pub price: Decimal,
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
    use rust_decimal_macros::dec; // For easy Decimal literal creation

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
            limit_price: Some(dec!(50000.50)),
            trigger_price: None,
            base_amount: dec!(1.5),
            remaining_base: dec!(1.5),
            filled_quote: dec!(0.0),
            filled_base: dec!(0.0),
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 1,
            remaining_quote: dec!(1.5) * dec!(50000.50),
        };
        assert_eq!(order.side, Side::Bid);
        assert_eq!(order.base_amount, dec!(1.5));
        assert_eq!(order.status, OrderStatus::New);
        assert_eq!(order.remaining_quote, dec!(75000.75));
    }

    #[test]
    fn test_trade_creation() {
        let now = Utc::now();
        let trade = Trade {
            id: Uuid::new_v4(),
            instrument_id: Uuid::new_v4(),
            maker_order_id: Uuid::new_v4(),
            taker_order_id: Uuid::new_v4(),
            base_amount: dec!(0.5),
            quote_amount: dec!(25000.25),
            price: dec!(50000.50),
            created_at: now,
        };
        assert_eq!(trade.base_amount, dec!(0.5));
        assert_eq!(trade.price, dec!(50000.50));
    }

    #[test]
    fn test_enum_derives() {
        // Test Side enum
        let bid_side = Side::Bid;
        let ask_side = Side::Ask;
        let cloned_bid = bid_side.clone();
        let copied_bid = bid_side;
        assert_eq!(bid_side, cloned_bid);
        assert_eq!(bid_side, copied_bid);
        assert_ne!(bid_side, ask_side);

        // Test OrderType enum
        let limit = OrderType::Limit;
        let market = OrderType::Market;
        let stop = OrderType::Stop;
        let stop_limit = OrderType::StopLimit;
        assert_ne!(limit, market);
        assert_ne!(stop, stop_limit);

        // Test OrderStatus enum
        let new = OrderStatus::New;
        let waiting = OrderStatus::WaitingTrigger;
        let partial = OrderStatus::PartiallyFilled;
        let filled = OrderStatus::Filled;
        let cancelled = OrderStatus::Cancelled;
        let partial_cancelled = OrderStatus::PartiallyFilledCancelled;
        assert_ne!(new, filled);
        assert_ne!(partial, cancelled);
        assert_ne!(waiting, partial_cancelled);

        // Test TriggerType enum
        let last_price = TriggerType::LastPrice;
        let cloned_trigger = last_price.clone();
        let copied_trigger = last_price;
        assert_eq!(last_price, cloned_trigger);
        assert_eq!(last_price, copied_trigger);

        // Test CreatedFrom enum
        let api = CreatedFrom::Api;
        let front = CreatedFrom::Front;
        let cloned_api = api.clone();
        let copied_api = api;
        assert_eq!(api, cloned_api);
        assert_eq!(api, copied_api);
        assert_ne!(api, front);
    }

    #[test]
    fn test_type_error() {
        let invalid_side = TypeError::InvalidSide("Invalid".to_string());
        let invalid_type = TypeError::InvalidOrderType("Invalid".to_string());
        
        // Test error messages
        assert_eq!(
            invalid_side.to_string(),
            "Invalid side specified: Invalid"
        );
        assert_eq!(
            invalid_type.to_string(),
            "Invalid order type specified: Invalid"
        );

        // Test cloning
        let cloned_side = invalid_side.clone();
        assert_eq!(invalid_side, cloned_side);
    }

    #[test]
    fn test_order_with_different_types() {
        let now = Utc::now();
        
        // Test Market order
        let market_order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("market-order-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Market,
            instrument_id: Uuid::new_v4(),
            side: Side::Ask,
            limit_price: None,
            trigger_price: None,
            base_amount: dec!(2.0),
            remaining_base: dec!(2.0),
            filled_quote: dec!(0.0),
            filled_base: dec!(0.0),
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Front,
            sequence_id: 2,
            remaining_quote: dec!(0.0), // Market orders don't have a price until execution
        };
        assert_eq!(market_order.order_type, OrderType::Market);
        assert_eq!(market_order.side, Side::Ask);
        assert_eq!(market_order.created_from, CreatedFrom::Front);

        // Test Stop order
        let stop_order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("stop-order-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Stop,
            instrument_id: Uuid::new_v4(),
            side: Side::Bid,
            limit_price: None,
            trigger_price: Some(dec!(45000.00)),
            base_amount: dec!(1.0),
            remaining_base: dec!(1.0),
            filled_quote: dec!(0.0),
            filled_base: dec!(0.0),
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::WaitingTrigger,
            created_at: now,
            updated_at: now,
            trigger_by: Some(TriggerType::LastPrice),
            created_from: CreatedFrom::Api,
            sequence_id: 3,
            remaining_quote: dec!(0.0), // Stop orders don't have a price until triggered
        };
        assert_eq!(stop_order.order_type, OrderType::Stop);
        assert_eq!(stop_order.status, OrderStatus::WaitingTrigger);
        assert_eq!(stop_order.trigger_by, Some(TriggerType::LastPrice));
    }

    #[test]
    fn test_order_status_transitions() {
        let now = Utc::now();
        
        // Test New -> PartiallyFilled transition
        let mut order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("partial-fill-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Limit,
            instrument_id: Uuid::new_v4(),
            side: Side::Bid,
            limit_price: Some(dec!(50000.00)),
            trigger_price: None,
            base_amount: dec!(1.0),
            remaining_base: dec!(0.5),
            filled_quote: dec!(25000.00),
            filled_base: dec!(0.5),
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::PartiallyFilled,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 4,
            remaining_quote: dec!(25000.00),
        };
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.remaining_base, dec!(0.5));
        assert_eq!(order.filled_base, dec!(0.5));

        // Test PartiallyFilled -> Filled transition
        order.remaining_base = dec!(0.0);
        order.filled_base = dec!(1.0);
        order.filled_quote = dec!(50000.00);
        order.remaining_quote = dec!(0.0);
        order.status = OrderStatus::Filled;
        assert_eq!(order.status, OrderStatus::Filled);
        assert_eq!(order.remaining_base, dec!(0.0));
        assert_eq!(order.filled_base, dec!(1.0));

        // Test New -> Cancelled transition
        let cancelled_order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("cancelled-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Limit,
            instrument_id: Uuid::new_v4(),
            side: Side::Ask,
            limit_price: Some(dec!(51000.00)),
            trigger_price: None,
            base_amount: dec!(1.0),
            remaining_base: dec!(1.0),
            filled_quote: dec!(0.0),
            filled_base: dec!(0.0),
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::Cancelled,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Front,
            sequence_id: 5,
            remaining_quote: dec!(51000.00),
        };
        assert_eq!(cancelled_order.status, OrderStatus::Cancelled);
        assert_eq!(cancelled_order.remaining_base, dec!(1.0));
        assert_eq!(cancelled_order.filled_base, dec!(0.0));

        // Test PartiallyFilled -> PartiallyFilledCancelled transition
        let partial_cancelled_order = Order {
            id: Uuid::new_v4(),
            ext_id: Some("partial-cancelled-1".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Limit,
            instrument_id: Uuid::new_v4(),
            side: Side::Bid,
            limit_price: Some(dec!(52000.00)),
            trigger_price: None,
            base_amount: dec!(1.0),
            remaining_base: dec!(0.3),
            filled_quote: dec!(36400.00),
            filled_base: dec!(0.7),
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::PartiallyFilledCancelled,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 6,
            remaining_quote: dec!(15600.00),
        };
        assert_eq!(partial_cancelled_order.status, OrderStatus::PartiallyFilledCancelled);
        assert_eq!(partial_cancelled_order.remaining_base, dec!(0.3));
        assert_eq!(partial_cancelled_order.filled_base, dec!(0.7));
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
            limit_price: Some(dec!(48000.00)),
            trigger_price: Some(dec!(47000.00)),
            base_amount: dec!(1.0),
            remaining_base: dec!(1.0),
            filled_quote: dec!(0.0),
            filled_base: dec!(0.0),
            expiration_date: now + chrono::Duration::days(365 * 2),
            status: OrderStatus::WaitingTrigger,
            created_at: now,
            updated_at: now,
            trigger_by: Some(TriggerType::LastPrice),
            created_from: CreatedFrom::Api,
            sequence_id: 7,
            remaining_quote: dec!(48000.00),
        };
        
        assert_eq!(stop_limit_order.order_type, OrderType::StopLimit);
        assert_eq!(stop_limit_order.status, OrderStatus::WaitingTrigger);
        assert_eq!(stop_limit_order.trigger_by, Some(TriggerType::LastPrice));
        assert_eq!(stop_limit_order.limit_price, Some(dec!(48000.00)));
        assert_eq!(stop_limit_order.trigger_price, Some(dec!(47000.00)));
    }
} 
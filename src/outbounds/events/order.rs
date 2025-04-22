//! Order-related events emitted by the matching engine.
//!
//! These events describe the lifecycle of orders processed by the matching engine,
//! including acknowledgements, rejections, executions and cancellations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use tracing::{debug, info};

use crate::domain::models::types::{Order, OrderStatus};
use crate::domain::models::orderbook::Match;

use super::market::{Level1Update, Level2Delta, OrderbookSnapshot, TradingSessionStatus};

/// +----------------------------------------------------------+
/// | STRUCTS | TRAITS | ENUMS | FUNCTIONS                     |
/// +----------+-------+-------+------------------------------+
/// | Enums:                                                   |
/// |   - EventOrderType                                       |
/// |   - EventOrderSide                                       |
/// |   - EventOrderStatus                                     |
/// |   - EventTimeInForce                                     |
/// |   - ResultEvent                                          |
/// | Structs:                                                 |
/// |   - OrderAcknowledgement                                 |
/// |   - OrderReject                                          |
/// |   - OrderCancel                                          |
/// |   - MatchEvent                                           |
/// |   - OrderUpdate                                          |
/// | Implementations:                                         |
/// |   - From<OrderType> for EventOrderType                   |
/// |   - From<Side> for EventOrderSide                        |
/// |   - From<OrderStatus> for EventOrderStatus              |
/// |   - From<TimeInForce> for EventTimeInForce              |
/// |   - From<Order> for OrderAcknowledgement                |
/// |   - From<Order> for OrderReject                         |
/// |   - From<Order> for OrderCancel                         |
/// |   - From<Match> for MatchEvent                          |
/// +----------------------------------------------------------+

// Re-export the types needed from domain models to avoid conflicts
use crate::domain::models::types::{Side, OrderType, TimeInForce};

/// Order types for event messages
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventOrderType {
    /// Limit order type
    Limit,
    
    /// Market order type
    Market,
    
    /// Stop order type
    Stop,
    
    /// Stop limit order type
    StopLimit,
}

/// Order sides for event messages
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventOrderSide {
    /// Buy/bid order
    Buy,
    
    /// Sell/ask order
    Sell,
}

/// Order status values for event messages
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventOrderStatus {
    /// Order is waiting for trigger condition
    WaitingTrigger,
    
    /// Order has been submitted but not yet processed
    Submitted,
    
    /// Order is active in the orderbook
    Unfilled,
    
    /// Order has been cancelled
    Cancelled,
    
    /// Order is partially filled
    PartiallyFilled,
    
    /// Order was partially filled then cancelled
    PartiallyFilledCancelled,
    
    /// Order has been completely filled
    Filled,
    
    /// Order was rejected
    Rejected,
}

/// Time-in-force values for event messages
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum EventTimeInForce {
    /// Good Till Cancelled
    Gtc,
    
    /// Immediate or Cancel
    Ioc,
    
    /// Fill or Kill
    Fok,
}

/// Display implementation for EventOrderType
impl fmt::Display for EventOrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventOrderType::Limit => write!(f, "LIMIT"),
            EventOrderType::Market => write!(f, "MARKET"),
            EventOrderType::Stop => write!(f, "STOP"),
            EventOrderType::StopLimit => write!(f, "STOP_LIMIT"),
        }
    }
}

/// Display implementation for EventOrderSide
impl fmt::Display for EventOrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventOrderSide::Buy => write!(f, "BUY"),
            EventOrderSide::Sell => write!(f, "SELL"),
        }
    }
}

/// Display implementation for EventOrderStatus
impl fmt::Display for EventOrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventOrderStatus::WaitingTrigger => write!(f, "WAITING_TRIGGER"),
            EventOrderStatus::Submitted => write!(f, "SUBMITTED"),
            EventOrderStatus::Unfilled => write!(f, "UNFILLED"),
            EventOrderStatus::Cancelled => write!(f, "CANCELLED"),
            EventOrderStatus::PartiallyFilled => write!(f, "PARTIALLY_FILLED"),
            EventOrderStatus::PartiallyFilledCancelled => write!(f, "PARTIALLY_FILLED_CANCELLED"),
            EventOrderStatus::Filled => write!(f, "FILLED"),
            EventOrderStatus::Rejected => write!(f, "REJECTED"),
        }
    }
}

/// Display implementation for EventTimeInForce
impl fmt::Display for EventTimeInForce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventTimeInForce::Gtc => write!(f, "GTC"),
            EventTimeInForce::Ioc => write!(f, "IOC"),
            EventTimeInForce::Fok => write!(f, "FOK"),
        }
    }
}

/// Conversion from domain OrderType to event EventOrderType
impl From<OrderType> for EventOrderType {
    fn from(order_type: OrderType) -> Self {
        match order_type {
            OrderType::Limit => EventOrderType::Limit,
            OrderType::Market => EventOrderType::Market,
            OrderType::Stop => EventOrderType::Stop,
            OrderType::StopLimit => EventOrderType::StopLimit,
        }
    }
}

/// Conversion from domain Side to event EventOrderSide
impl From<Side> for EventOrderSide {
    fn from(side: Side) -> Self {
        match side {
            Side::Bid => EventOrderSide::Buy,
            Side::Ask => EventOrderSide::Sell,
        }
    }
}

/// Conversion from domain OrderStatus to event EventOrderStatus
impl From<OrderStatus> for EventOrderStatus {
    fn from(status: OrderStatus) -> Self {
        match status {
            OrderStatus::Submitted => EventOrderStatus::Submitted,
            OrderStatus::Unfilled => EventOrderStatus::Unfilled,
            OrderStatus::PartiallyFilled => EventOrderStatus::PartiallyFilled,
            OrderStatus::PartiallyFilledCancelled => EventOrderStatus::PartiallyFilledCancelled,
            OrderStatus::Filled => EventOrderStatus::Filled,
            OrderStatus::WaitingTrigger => EventOrderStatus::WaitingTrigger,
            OrderStatus::Cancelled => EventOrderStatus::Cancelled,
            OrderStatus::Rejected => EventOrderStatus::Rejected,
        }
    }
}

/// Conversion from domain TimeInForce to event EventTimeInForce
impl From<TimeInForce> for EventTimeInForce {
    fn from(tif: TimeInForce) -> Self {
        match tif {
            TimeInForce::GTC => EventTimeInForce::Gtc,
            TimeInForce::IOC => EventTimeInForce::Ioc,
        }
    }
}

/// Order acknowledgement message
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct OrderAcknowledgement {
    /// Order identifier
    pub order_id: Uuid,
    
    /// New status of the order
    pub new_status: EventOrderStatus,
    
    /// Sequence number for event ordering
    pub seq_num: u64,
    
    /// When the acknowledgement was created
    pub timestamp: DateTime<Utc>,
}

/// Represents a rejection of an order by the matching engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderReject {
    /// ID of the order that was rejected
    pub order_id: Uuid,
    /// Symbol/instrument for the order
    pub symbol: String,
    /// Reason for the rejection
    pub reason: String,
    /// Timestamp when the order was rejected
    pub timestamp: DateTime<Utc>,
    /// Sequence number for this event
    pub seq_num: u64,
    /// Error code explaining the rejection reason
    pub error_code: u32,
    /// New status of the order (always Rejected)
    pub new_status: EventOrderStatus,
}

/// Represents a cancellation of an order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderCancel {
    /// ID of the order that was cancelled
    pub order_id: Uuid,
    /// Symbol/instrument for the order
    pub symbol: String,
    /// Reason for the cancellation
    pub reason: String,
    /// Timestamp when the order was cancelled
    pub timestamp: DateTime<Utc>,
    /// Sequence number for this event
    pub seq_num: u64,
    /// Base amount filled before cancellation
    pub filled_base: u64,
    /// Quote amount filled before cancellation
    pub filled_quote: u64,
    /// Remaining quantity that was cancelled (optional)
    pub remaining_quantity: Option<u64>,
}

/// Trade match event
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct MatchEvent {
    /// Aggressive order ID
    pub taker_order_id: Uuid,
    
    /// Passive order ID
    pub maker_order_id: Uuid,
    
    /// Account ID of the taker
    pub taker_account_id: Uuid,
    
    /// Account ID of the maker
    pub maker_account_id: Uuid,
    
    /// New status of the maker order
    pub maker_status: EventOrderStatus,
    
    /// New status of the taker order
    pub taker_status: EventOrderStatus,
    
    /// Base quantity matched
    pub match_base_amount: u64,
    
    /// Quote quantity matched
    pub match_quote_amount: u64,
    
    /// When the match occurred
    pub timestamp: DateTime<Utc>,
    
    /// Sequence number for event ordering
    pub seq_num: u64,
    
    /// Price at which the match occurred
    pub match_price: i64,
}

/// Order update event
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct OrderUpdate {
    /// Order identifier
    pub order_id: Uuid,
    
    /// Account owning the order
    pub account_id: Uuid,
    
    /// Base amount filled
    pub filled_base: u64,
    
    /// Quote amount filled
    pub filled_quote: u64,
    
    /// Remaining base amount
    pub remaining_base: u64,
    
    /// Sequence number for event ordering
    pub seq_num: u64,
    
    /// Current status of the order
    pub order_status: EventOrderStatus,
    
    /// When the update was created
    pub timestamp: DateTime<Utc>,
}

/// Enumeration of all possible result events
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum ResultEvent {
    /// Match between orders
    #[serde(rename = "MATCH")]
    Match(MatchEvent),
    
    /// Order acknowledgement
    #[serde(rename = "ORDER_ACK")]
    OrderAck(OrderAcknowledgement),
    
    /// Order cancellation
    #[serde(rename = "ORDER_CANCEL")]
    OrderCancel(OrderCancel),
    
    /// Order rejection
    #[serde(rename = "ORDER_REJECT")]
    OrderReject(OrderReject),
    
    /// Order update
    #[serde(rename = "ORDER_UPDATE")]
    OrderUpdate(OrderUpdate),
    
    /// Level 1 market data update
    #[serde(rename = "L1_UPDATE")]
    L1Update(Level1Update),
    
    /// Level 2 market data update
    #[serde(rename = "L2_DELTA")]
    L2Delta(Level2Delta),
    
    /// Trading session status
    #[serde(rename = "TRADING_STATUS")]
    TradingStatus(TradingSessionStatus),
    
    /// Orderbook snapshot
    #[serde(rename = "SNAPSHOT")]
    Snapshot(OrderbookSnapshot),
}

/// Conversion from domain Order to event OrderAcknowledgement
impl From<Order> for OrderAcknowledgement {
    fn from(order: Order) -> Self {
        Self {
            order_id: order.id,
            new_status: EventOrderStatus::from(order.status),
            seq_num: 0, // Should be assigned by the event manager
            timestamp: Utc::now(),
        }
    }
}

/// Conversion from domain Order to event OrderReject
impl From<Order> for OrderReject {
    fn from(order: Order) -> Self {
        Self {
            order_id: order.id,
            timestamp: Utc::now(),
            seq_num: 0, // Should be assigned by the event manager
            error_code: 1, // Default error code, should be overridden
            new_status: EventOrderStatus::Rejected,
            reason: String::new(),
            symbol: String::new(),
        }
    }
}

/// Conversion from domain Order to event OrderCancel
impl From<Order> for OrderCancel {
    fn from(order: Order) -> Self {
        Self {
            order_id: order.id,
            timestamp: Utc::now(),
            seq_num: 0, // Should be assigned by the event manager
            filled_base: order.filled_base,
            filled_quote: order.filled_quote,
            reason: String::new(),
            remaining_quantity: None,
            symbol: String::new(),
        }
    }
}

/// Conversion from domain Order to event OrderUpdate
impl From<Order> for OrderUpdate {
    fn from(order: Order) -> Self {
        Self {
            order_id: order.id,
            account_id: order.account_id,
            filled_base: order.filled_base,
            filled_quote: order.filled_quote,
            remaining_base: order.remaining_base,
            seq_num: 0, // Should be assigned by the event manager
            order_status: EventOrderStatus::from(order.status),
            timestamp: Utc::now(),
        }
    }
}

/// Conversion from domain Match to event MatchEvent
impl From<Match> for MatchEvent {
    fn from(match_data: Match) -> Self {
        Self {
            taker_order_id: match_data.taker_order_id,
            maker_order_id: match_data.maker_order_id,
            taker_account_id: match_data.taker_account_id,
            maker_account_id: match_data.maker_account_id,
            maker_status: EventOrderStatus::from(match_data.maker_status),
            taker_status: EventOrderStatus::from(match_data.taker_status),
            match_base_amount: match_data.match_base_amount,
            match_quote_amount: match_data.match_quote_amount,
            timestamp: Utc::now(),
            seq_num: match_data.seq_num,
            match_price: match_data.limit_price,
        }
    }
}

/// Represents a trade or execution that has occurred in the matching engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Execution {
    /// Unique execution ID
    pub execution_id: Uuid,
    /// ID of the order that was executed
    pub order_id: Uuid,
    /// Symbol/instrument that was traded
    pub symbol: String,
    /// Price at which the execution occurred
    pub price: u64,
    /// Quantity that was executed
    pub quantity: u64,
    /// Side of the trade (from taker's perspective)
    pub side: Side,
    /// Timestamp when the execution occurred
    pub timestamp: DateTime<Utc>,
    /// Sequence number for this event
    pub sequence: u64,
    /// Whether this execution resulted in a self-match
    pub is_self_match: bool,
}

/// Represents an acknowledgement of an order received by the matching engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderAck {
    /// ID of the order that was acknowledged
    pub order_id: Uuid,
    /// Symbol/instrument for the order
    pub symbol: String,
    /// Side of the order (buy or sell)
    pub side: Side,
    /// Price of the order (for limit orders)
    pub price: Option<u64>,
    /// Quantity of the order
    pub quantity: u64,
    /// Type of the order (market or limit)
    pub order_type: OrderType,
    /// Time-in-force for the order
    pub time_in_force: TimeInForce,
    /// Timestamp when the order was acknowledged
    pub timestamp: DateTime<Utc>,
    /// Sequence number for this event
    pub sequence: u64,
}

/// +-------+
/// | ENUMS |
/// +-------+

/// Represents all possible order events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum OrderEvent {
    /// Order acknowledged
    OrderAck(OrderAck),
    /// Order rejected
    OrderReject(OrderReject),
    /// Order cancelled
    OrderCancel(OrderCancel),
    /// Order executed
    Execution(Execution),
}

/// Handles order-related events from the matching engine
#[derive(Debug, Clone)]
pub struct OrderEventHandler {
    /// Whether to log events to the console
    log_to_console: bool,
}

impl OrderEventHandler {
    /// Creates a new order event handler
    pub fn new() -> Self {
        Self {
            log_to_console: true,
        }
    }
    
    /// Creates a new order event handler with console logging disabled
    pub fn new_silent() -> Self {
        Self {
            log_to_console: false,
        }
    }
    
    /// Handles an order added event
    pub fn handle_order_added(&self, order: &Order) {
        if self.log_to_console {
            info!("Order added: {} ({})", order.id, order.side);
        }
        debug!("Order added: {:?}", order);
        
        // Here you would typically:
        // 1. Forward to external systems (e.g. WebSocket, Kafka)
        // 2. Persist the event to a database
        // 3. Notify other components that need to react to this event
    }
    
    /// Handles an order matched event
    pub fn handle_order_matched(&self, order: &Order, matched_quantity: u64) {
        if self.log_to_console {
            info!("Order matched: {} ({}), quantity: {}", order.id, order.side, matched_quantity);
        }
        debug!("Order matched: {:?}, quantity: {}", order, matched_quantity);
        
        // Here you would typically:
        // 1. Forward to external systems
        // 2. Persist the event
        // 3. Notify other components
    }
    
    /// Handles an order cancelled event
    pub fn handle_order_cancelled(&self, order: &Order) {
        if self.log_to_console {
            info!("Order cancelled: {} ({})", order.id, order.side);
        }
        debug!("Order cancelled: {:?}", order);
        
        // Here you would typically:
        // 1. Forward to external systems
        // 2. Persist the event
        // 3. Notify other components
    }
    
    /// Handles an order status changed event
    pub fn handle_order_status_changed(&self, order_id: &str, previous_status: OrderStatus, new_status: OrderStatus) {
        if self.log_to_console {
            info!("Order status changed: {} from {:?} to {:?}", order_id, previous_status, new_status);
        }
        debug!("Order status changed: {} from {:?} to {:?}", order_id, previous_status, new_status);
        
        // Here you would typically:
        // 1. Forward to external systems
        // 2. Persist the event
        // 3. Notify other components
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_match_event_serialization() {
        let match_event = MatchEvent {
            taker_order_id: Uuid::new_v4(),
            maker_order_id: Uuid::new_v4(),
            taker_account_id: Uuid::new_v4(),
            maker_account_id: Uuid::new_v4(),
            maker_status: EventOrderStatus::PartiallyFilled,
            taker_status: EventOrderStatus::Filled,
            match_base_amount: 100_000,
            match_quote_amount: 5_000_000,
            timestamp: Utc::now(),
            seq_num: 123,
            match_price: 50_000,
        };
        
        let json = serde_json::to_string(&match_event).unwrap();
        let deserialized: MatchEvent = serde_json::from_str(&json).unwrap();
        
        // Compare fields that don't depend on exact timestamp
        assert_eq!(match_event.taker_order_id, deserialized.taker_order_id);
        assert_eq!(match_event.maker_order_id, deserialized.maker_order_id);
        assert_eq!(match_event.match_base_amount, deserialized.match_base_amount);
        assert_eq!(match_event.match_quote_amount, deserialized.match_quote_amount);
        assert_eq!(match_event.match_price, deserialized.match_price);
    }
    
    #[test]
    fn test_order_status_conversion() {
        let statuses = vec![
            (OrderStatus::Submitted, EventOrderStatus::Submitted),
            (OrderStatus::Unfilled, EventOrderStatus::Unfilled),
            (OrderStatus::PartiallyFilled, EventOrderStatus::PartiallyFilled),
            (OrderStatus::Filled, EventOrderStatus::Filled),
            (OrderStatus::Cancelled, EventOrderStatus::Cancelled),
            (OrderStatus::Rejected, EventOrderStatus::Rejected),
        ];
        
        for (domain_status, expected_event_status) in statuses {
            let event_status = EventOrderStatus::from(domain_status);
            assert_eq!(event_status, expected_event_status);
        }
    }
} 
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use crate::types::{Order, OrderStatus, Trade, Side};

/// Represents different event types in the trading system
#[derive(Debug, Clone)]
pub enum Event {
    /// Events related to order lifecycle
    Order(OrderEvent),
    /// Events related to trades
    Trade(TradeEvent),
    /// Events related to order book changes
    OrderBook(OrderBookEvent),
    /// Events related to market data
    Market(MarketEvent),
}

/// Events related to orders
#[derive(Debug, Clone)]
pub enum OrderEvent {
    /// A new order has been received
    Created(Order),
    /// An order's status has changed
    StatusChanged {
        order_id: Uuid,
        instrument_id: Uuid,
        old_status: OrderStatus,
        new_status: OrderStatus,
    },
    /// An order has been modified
    Modified {
        old_order: Order,
        new_order: Order,
    },
    /// An order has been cancelled
    Cancelled(Order),
    /// An order has been rejected
    Rejected {
        order_id: Uuid,
        instrument_id: Uuid,
        reason: String,
    },
}

/// Events related to trades
#[derive(Debug, Clone)]
pub enum TradeEvent {
    /// A new trade has been executed
    Executed(Trade),
}

/// Events related to order book changes
#[derive(Debug, Clone)]
pub enum OrderBookEvent {
    /// Price level added to the order book
    LevelAdded {
        instrument_id: Uuid,
        side: Side,
        price: Decimal,
        volume: Decimal,
    },
    /// Price level removed from the order book
    LevelRemoved {
        instrument_id: Uuid,
        side: Side,
        price: Decimal,
    },
    /// Price level updated in the order book
    LevelUpdated {
        instrument_id: Uuid,
        side: Side,
        price: Decimal,
        old_volume: Decimal,
        new_volume: Decimal,
    },
    /// Best prices changed
    BestPricesChanged {
        instrument_id: Uuid,
        old_bid: Option<Decimal>,
        new_bid: Option<Decimal>,
        old_ask: Option<Decimal>,
        new_ask: Option<Decimal>,
    },
}

/// Events related to market data
#[derive(Debug, Clone)]
pub enum MarketEvent {
    /// New price tick
    PriceTick {
        instrument_id: Uuid,
        price: Decimal,
        timestamp: DateTime<Utc>,
    },
    /// Trading status changed
    StatusChanged {
        instrument_id: Uuid,
        is_trading: bool,
        reason: Option<String>,
    },
}

/// Metadata for events
#[derive(Debug, Clone)]
pub struct EventMetadata {
    /// Unique identifier for the event
    pub id: Uuid,
    /// Timestamp when the event was created
    pub timestamp: DateTime<Utc>,
    /// Sequence number for ordering events
    pub sequence: u64,
    /// Source component that generated the event
    pub source: String,
} 
//! Market-related events emitted by the matching engine.
//!
//! These events describe changes to market data and status information
//! that is of interest to market participants.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::domain::models::types::Trade;
use crate::domain::services::orderbook::depth::DepthSnapshot;
use tracing::{debug, info};

/// +----------------------------------------------------------+
/// | STRUCTS | TRAITS | ENUMS | FUNCTIONS                     |
/// +----------+-------+-------+------------------------------+
/// | Structs:                                                 |
/// |   - Level1Update                                         |
/// |   - Level2Delta                                          |
/// |   - OrderbookSnapshot                                    |
/// |   - TradingSessionStatus                                 |
/// | Enums:                                                   |
/// |   - MarketEventType                                      |
/// |   - TradingStatus                                        |
/// |   - MessageType                                          |
/// +----------------------------------------------------------+

/// Types of market events published by the matching engine.
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MarketEventType {
    /// Top of book (best bid/ask) update
    Level1,
    
    /// Order book depth update with price/quantity deltas
    Level2,
    
    /// Complete order book snapshot
    Snapshot,
    
    /// Trading session status update
    TradingSessionStatus,
}

/// Trading session status values
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TradingStatus {
    /// Trading is active
    Running,
    
    /// Trading is temporarily paused
    Halted,
    
    /// Trading has been stopped
    Stopped,
    
    /// Market is open and trading normally
    Trading,
    
    /// Market is in pre-open state
    PreOpen,
    
    /// Market is closed
    Closed,
}

/// Types of messages accepted during different trading states
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    /// New order placement messages
    NewOrder,
    
    /// Order cancellation messages
    CancelOrder,
}

/// Top-of-book update event (best bid and ask)
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Level1Update {
    /// Protocol version number
    pub version: u32,
    
    /// Event type identifier
    pub event_type: MarketEventType,
    
    /// Best bid price and size (can be None if no bid)
    pub bid: Option<(i64, u64)>,
    
    /// Best ask price and size (can be None if no ask)
    pub ask: Option<(i64, u64)>,
    
    /// Monotonically increasing sequence number
    pub seq_num: u64,
    
    /// Unix timestamp when the update was generated
    pub timestamp: u64,
    
    /// Instrument identifier
    pub instrument_id: Uuid,
}

/// Order book depth update
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Level2Delta {
    /// Protocol version number
    pub version: u32,
    
    /// Event type identifier
    pub event_type: MarketEventType,
    
    /// Vector of (price, quantity) tuples for bids
    pub bids: Vec<(i64, u64)>,
    
    /// Vector of (price, quantity) tuples for asks
    pub asks: Vec<(i64, u64)>,
    
    /// Monotonically increasing sequence number
    pub seq_num: u64,
    
    /// Unix timestamp when the update was generated
    pub timestamp: u64,
    
    /// Instrument identifier
    pub instrument_id: Uuid,
}

/// Complete order book snapshot
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct OrderbookSnapshot {
    /// Protocol version number
    pub version: u32,
    
    /// Event type identifier
    pub event_type: MarketEventType,
    
    /// Vector of (price, quantity) tuples for bids
    pub bids: Vec<(i64, u64)>,
    
    /// Vector of (price, quantity) tuples for asks
    pub asks: Vec<(i64, u64)>,
    
    /// Monotonically increasing sequence number
    pub seq_num: u64,
    
    /// Timestamp when the snapshot was generated
    pub timestamp: DateTime<Utc>,
    
    /// Instrument identifier
    pub instrument_id: Uuid,
}

/// Trading session status update
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TradingSessionStatus {
    /// Protocol version number
    pub version: u32,
    
    /// Event type identifier
    pub event_type: MarketEventType,
    
    /// Timestamp when the status change occurred
    pub timestamp: DateTime<Utc>,
    
    /// Current trading status
    pub status: TradingStatus,
    
    /// Messages accepted in current status
    pub accepted_messages: Vec<MessageType>,
    
    /// Instrument identifier
    pub instrument_id: Uuid,
}

/// Represents a price level in the order book with aggregated quantity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PriceLevel {
    /// Price of this level
    pub price: u64,
    /// Total quantity available at this price
    pub quantity: u64,
    /// Number of orders at this price level
    pub order_count: u32,
}

/// Represents the best bid and ask prices in the market
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct L1Data {
    /// Symbol identifier
    pub symbol: String,
    /// Best bid price level
    pub bid: Option<PriceLevel>,
    /// Best ask price level
    pub ask: Option<PriceLevel>,
    /// Timestamp when this data was generated
    pub timestamp: DateTime<Utc>,
    /// Sequence number for this event
    pub sequence: u64,
}

/// Represents a snapshot of order book price levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct L2Snapshot {
    /// Symbol identifier
    pub symbol: String,
    /// Array of bid price levels sorted by price (descending)
    pub bids: Vec<PriceLevel>,
    /// Array of ask price levels sorted by price (ascending)
    pub asks: Vec<PriceLevel>,
    /// Timestamp when this snapshot was generated
    pub timestamp: DateTime<Utc>,
    /// Sequence number for this event
    pub sequence: u64,
}

/// Represents a change to a specific price level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct L2Update {
    /// Symbol identifier
    pub symbol: String,
    /// Updates to bid levels
    pub bid_updates: Vec<PriceLevelUpdate>,
    /// Updates to ask levels
    pub ask_updates: Vec<PriceLevelUpdate>,
    /// Timestamp when this update was generated
    pub timestamp: DateTime<Utc>,
    /// Sequence number for this event
    pub sequence: u64,
}

/// Describes the type of update to a price level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateType {
    /// New price level added
    New,
    /// Existing price level updated
    Update,
    /// Price level removed
    Delete,
}

/// Represents an update to a specific price level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PriceLevelUpdate {
    /// Price of the level being updated
    pub price: u64,
    /// New quantity (0 for Delete)
    pub quantity: u64,
    /// Number of orders at this price level
    pub order_count: u32,
    /// Type of update
    pub update_type: UpdateType,
}

/// Event indicating a change in trading status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradingStatusUpdate {
    /// Symbol identifier
    pub symbol: String,
    /// New trading status
    pub status: TradingStatus,
    /// Reason for the status change
    pub reason: String,
    /// Timestamp when this status change occurred
    pub timestamp: DateTime<Utc>,
    /// Sequence number for this event
    pub sequence: u64,
}

/// Represents all possible market events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum MarketEvent {
    /// L1 market data update
    L1Update(L1Data),
    /// L2 market data snapshot
    L2Snapshot(L2Snapshot),
    /// L2 market data update
    L2Update(L2Update),
    /// Market trading status change
    StatusUpdate(TradingStatusUpdate),
}

/// Handles market-related events from the matching engine
#[derive(Debug, Clone)]
pub struct MarketEventHandler {
    /// Whether to log events to the console
    log_to_console: bool,
}

impl MarketEventHandler {
    /// Creates a new market event handler
    pub fn new() -> Self {
        Self {
            log_to_console: true,
        }
    }
    
    /// Creates a new market event handler with console logging disabled
    pub fn new_silent() -> Self {
        Self {
            log_to_console: false,
        }
    }
    
    /// Handles a trade executed event
    pub fn handle_trade_executed(&self, trade: &Trade) {
        if self.log_to_console {
            info!("Trade executed: {}, price: {}, quantity: {}", 
                  trade.id, trade.price, trade.base_amount);
        }
        debug!("Trade executed: {:?}", trade);
        
        // Here you would typically:
        // 1. Forward to external systems (e.g. WebSocket, Kafka)
        // 2. Persist the trade to a database
        // 3. Notify other components that need to react to trades
    }
    
    /// Handles a depth updated event
    pub fn handle_depth_updated(&self, depth: &DepthSnapshot) {
        if self.log_to_console {
            let bid_levels = depth.bids.len();
            let ask_levels = depth.asks.len();
            
            info!("Depth updated: {} bid levels, {} ask levels for instrument {}", 
                  bid_levels, ask_levels, depth.instrument_id);
        }
        debug!("Depth updated: {:?}", depth);
        
        // Here you would typically:
        // 1. Forward to external systems
        // 2. Update market data systems
        // 3. Notify clients of depth changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    #[test]
    fn test_level1_update_serialization() {
        let update = Level1Update {
            version: 1,
            event_type: MarketEventType::Level1,
            bid: Some((100_000, 500_000)),
            ask: Some((101_000, 300_000)),
            seq_num: 42,
            timestamp: 1617234567890,
            instrument_id: Uuid::new_v4(),
        };
        
        let json = serde_json::to_string(&update).unwrap();
        let deserialized: Level1Update = serde_json::from_str(&json).unwrap();
        
        assert_eq!(update, deserialized);
    }
    
    #[test]
    fn test_level2_delta_serialization() {
        let delta = Level2Delta {
            version: 1,
            event_type: MarketEventType::Level2,
            bids: vec![(100_000, 500_000), (99_000, 750_000)],
            asks: vec![(101_000, 300_000), (102_000, 600_000)],
            seq_num: 43,
            timestamp: 1617234567891,
            instrument_id: Uuid::new_v4(),
        };
        
        let json = serde_json::to_string(&delta).unwrap();
        let deserialized: Level2Delta = serde_json::from_str(&json).unwrap();
        
        assert_eq!(delta, deserialized);
    }
} 
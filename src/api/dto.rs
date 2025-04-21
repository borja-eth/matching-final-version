//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name                 | Description                               | Key Methods         |
// |----------------------|-------------------------------------------|---------------------|
// | CreateOrderRequest   | Request to create an order                | from_request        |
// | OrderResponse        | Order response with full details          | from_order          |
// | DepthResponse        | Order book depth response                 | from_snapshot       |
// | OrderBookResponse    | Complete order book response              | from_orderbook      |
// | TradeResponse        | Trade response                            | from_trade          |
//--------------------------------------------------------------------------------------------------

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::types::{Order, Side, OrderType, OrderStatus, Trade, TimeInForce};
use crate::depth::{DepthSnapshot, PriceLevel};

/// Request to create a new order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderRequest {
    /// Optional external identifier provided by the client
    pub ext_id: Option<String>,
    /// Identifier for the account placing the order
    pub account_id: Uuid,
    /// Type of the order (Limit, Market, etc.)
    pub order_type: OrderType,
    /// Identifier for the instrument being traded
    pub instrument_id: Uuid,
    /// Side of the order (Buy or Sell)
    pub side: Side,
    /// Limit price for Limit/StopLimit orders
    pub limit_price: Option<Decimal>,
    /// Trigger price for Stop/StopLimit orders
    pub trigger_price: Option<Decimal>,
    /// Initial order quantity in base units
    pub base_amount: Decimal,
    /// Time-in-force policy for the order
    #[serde(default)]
    pub time_in_force: TimeInForce,
}

impl CreateOrderRequest {
    /// Converts the request into an Order with default values
    pub fn into_order(self) -> Order {
        let now = Utc::now();
        
        Order {
            id: Uuid::new_v4(),
            ext_id: self.ext_id,
            account_id: self.account_id,
            order_type: self.order_type,
            instrument_id: self.instrument_id,
            side: self.side,
            limit_price: self.limit_price.map(|p| ((p * Decimal::from(100000)).round().to_string().parse::<i64>().unwrap_or(0))),
            trigger_price: self.trigger_price.map(|p| ((p * Decimal::from(100000)).round().to_string().parse::<i64>().unwrap_or(0))),
            base_amount: (self.base_amount * Decimal::from(100000)).round().to_string().parse::<u64>().unwrap_or(0),
            remaining_base: (self.base_amount * Decimal::from(100000)).round().to_string().parse::<u64>().unwrap_or(0),
            filled_base: 0,
            remaining_quote: self.limit_price.map_or(0, |p| ((p * self.base_amount * Decimal::from(100000)).round().to_string().parse::<u64>().unwrap_or(0))),
            filled_quote: 0,
            expiration_date: now + chrono::Duration::days(7), // Default 7-day expiration
            status: OrderStatus::Submitted,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: crate::types::CreatedFrom::Api,
            sequence_id: 0, // Will be set by the engine
            time_in_force: self.time_in_force,
        }
    }
}

/// Response for an order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    /// Unique identifier for the order
    pub id: Uuid,
    /// Optional external identifier provided by the client
    pub ext_id: Option<String>,
    /// Identifier for the account that placed the order
    pub account_id: Uuid,
    /// Type of the order
    pub order_type: OrderType,
    /// Identifier for the instrument being traded
    pub instrument_id: Uuid,
    /// Side of the order
    pub side: Side,
    /// Limit price for Limit/StopLimit orders
    pub limit_price: Option<Decimal>,
    /// Trigger price for Stop/StopLimit orders
    pub trigger_price: Option<Decimal>,
    /// Initial order quantity in base units
    pub base_amount: Decimal,
    /// Remaining quantity in base units
    pub remaining_base: Decimal,
    /// Filled quantity in base units
    pub filled_base: Decimal,
    /// Filled quantity in quote units
    pub filled_quote: Decimal,
    /// Current status of the order
    pub status: OrderStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl From<Order> for OrderResponse {
    fn from(order: Order) -> Self {
        Self {
            id: order.id,
            ext_id: order.ext_id,
            account_id: order.account_id,
            order_type: order.order_type,
            instrument_id: order.instrument_id,
            side: order.side,
            limit_price: order.limit_price.map(|p| Decimal::from(p) / Decimal::from(100000)),
            trigger_price: order.trigger_price.map(|p| Decimal::from(p) / Decimal::from(100000)),
            base_amount: Decimal::from(order.base_amount) / Decimal::from(100000),
            remaining_base: Decimal::from(order.remaining_base) / Decimal::from(100000),
            filled_base: Decimal::from(order.filled_base) / Decimal::from(100000),
            filled_quote: Decimal::from(order.filled_quote) / Decimal::from(100000),
            status: order.status,
            created_at: order.created_at,
            updated_at: order.updated_at,
        }
    }
}

/// Price level in the depth response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevelResponse {
    /// Price for this level
    pub price: Decimal,
    /// Total volume at this price level
    pub volume: Decimal,
    /// Number of orders at this price level
    pub order_count: u32,
}

impl From<PriceLevel> for PriceLevelResponse {
    fn from(level: PriceLevel) -> Self {
        Self {
            price: Decimal::from(level.price) / Decimal::from(100000),
            volume: Decimal::from(level.volume) / Decimal::from(100000),
            order_count: level.order_count,
        }
    }
}

/// Response for order book depth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthResponse {
    /// Bid side price levels (descending order by price)
    pub bids: Vec<PriceLevelResponse>,
    /// Ask side price levels (ascending order by price)
    pub asks: Vec<PriceLevelResponse>,
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
    /// Instrument ID
    pub instrument_id: Uuid,
}

impl From<DepthSnapshot> for DepthResponse {
    fn from(snapshot: DepthSnapshot) -> Self {
        Self {
            bids: snapshot.bids.into_iter().map(PriceLevelResponse::from).collect(),
            asks: snapshot.asks.into_iter().map(PriceLevelResponse::from).collect(),
            timestamp: snapshot.timestamp,
            instrument_id: snapshot.instrument_id,
        }
    }
}

/// Response for a trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResponse {
    /// Unique identifier for the trade
    pub id: Uuid,
    /// Identifier for the instrument traded
    pub instrument_id: Uuid,
    /// ID of the maker order
    pub maker_order_id: Uuid,
    /// ID of the taker order
    pub taker_order_id: Uuid,
    /// Quantity traded in base units
    pub base_amount: Decimal,
    /// Quantity traded in quote units
    pub quote_amount: Decimal,
    /// Price at which the trade occurred
    pub price: Decimal,
    /// Timestamp when the trade occurred
    pub created_at: DateTime<Utc>,
}

impl From<Trade> for TradeResponse {
    fn from(trade: Trade) -> Self {
        Self {
            id: trade.id,
            instrument_id: trade.instrument_id,
            maker_order_id: trade.maker_order_id,
            taker_order_id: trade.taker_order_id,
            base_amount: Decimal::from(trade.base_amount) / Decimal::from(100000),
            quote_amount: Decimal::from(trade.quote_amount) / Decimal::from(100000),
            price: Decimal::from(trade.price) / Decimal::from(100000),
            created_at: trade.created_at,
        }
    }
}

/// Request to create a new instrument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstrumentRequest {
    /// Optional specific ID for the instrument (random if not provided)
    pub id: Option<Uuid>,
    /// Human-readable name for the instrument
    pub name: String,
    /// Base currency symbol
    pub base_currency: String,
    /// Quote currency symbol
    pub quote_currency: String,
}

/// Response for an instrument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentResponse {
    /// Unique identifier for the instrument
    pub id: Uuid,
    /// Human-readable name for the instrument
    pub name: String,
    /// Base currency symbol
    pub base_currency: String,
    /// Quote currency symbol
    pub quote_currency: String,
} 
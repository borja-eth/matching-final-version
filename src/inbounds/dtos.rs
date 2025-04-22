use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::models::types::{Order, OrderType, Side, TimeInForce, CreatedFrom, OrderStatus};

/// +----------------------------------------------------------+
/// | STRUCTS | TRAITS | ENUMS | FUNCTIONS                     |
/// +----------+-------+-------+------------------------------+
/// | Structs:                                                 |
/// |   - PlaceOrderRequest                                    |
/// |   - CancelOrderRequest                                   |
/// |   - SnapshotRequest                                      |
/// |   - TradingStatusRequest                                 |
/// | Implementations:                                         |
/// |   - From<PlaceOrderRequest> for Order                    |
/// +----------------------------------------------------------+

/// Request to place a new order in the matching engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrderRequest {
    /// Version of the request format.
    pub version: u32,
    
    /// Type of request (always "place").
    pub request_type: String,
    
    /// ID of the instrument to place the order on.
    pub instrument: Uuid,
    
    /// ID for the new order.
    pub new_order_id: Uuid,
    
    /// ID of the account placing the order.
    pub account_id: Uuid,
    
    /// Side of the order (buy/sell).
    pub side: Side,
    
    /// Type of the order (limit, market, etc.).
    pub order_type: OrderType,
    
    /// Limit price for limit orders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<i64>,
    
    /// Amount of the base asset to trade.
    pub base_amount: u64,
    
    /// Optional trigger price for stop orders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<i64>,
    
    /// Time in force setting for the order.
    #[serde(default)]
    pub time_in_force: TimeInForce,
    
    /// Optional client-provided external ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext_id: Option<String>,
}

/// Request to cancel an existing order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderRequest {
    /// Version of the request format.
    pub version: u32,
    
    /// Type of request (always "cancel").
    pub request_type: String,
    
    /// ID of the instrument the order is on.
    pub instrument: Uuid,
    
    /// ID of the order to cancel.
    pub order_id: Uuid,
}

/// Request for an orderbook snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRequest {
    /// ID of the instrument to get a snapshot for.
    pub instrument: Uuid,
}

/// Request for trading status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingStatusRequest {
    /// ID of the instrument to get status for.
    pub instrument: Uuid,
}

impl From<PlaceOrderRequest> for Order {
    fn from(req: PlaceOrderRequest) -> Self {
        let now = Utc::now();
        
        // Default quote value calculation based on available data
        let remaining_quote = match (req.limit_price, req.base_amount) {
            (Some(price), amount) => price as u64 * amount,
            _ => 0, // For market orders, we don't know yet
        };
        
        Order {
            id: req.new_order_id,
            ext_id: req.ext_id,
            account_id: req.account_id,
            side: req.side,
            order_type: req.order_type,
            limit_price: req.limit_price,
            base_amount: req.base_amount,
            remaining_base: req.base_amount,
            time_in_force: req.time_in_force,
            created_at: now,
            status: OrderStatus::Submitted,
            instrument_id: req.instrument,
            trigger_price: req.trigger_price,
            remaining_quote,
            filled_quote: 0,
            filled_base: 0,
            expiration_date: now + chrono::Duration::days(30), // Default 30 day expiration
            updated_at: now,
            trigger_by: None, // Not supported in this DTO version
            created_from: CreatedFrom::Api,
            sequence_id: 0, // Will be assigned by the orderbook
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_place_order_request_conversion() {
        let place_request = PlaceOrderRequest {
            version: 1,
            request_type: "place".to_string(),
            instrument: Uuid::new_v4(),
            new_order_id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            side: Side::Bid,
            order_type: OrderType::Limit,
            limit_price: Some(10000),
            base_amount: 100000,
            trigger_price: None,
            time_in_force: TimeInForce::GTC,
            ext_id: Some("client-123".to_string()),
        };
        
        let order = Order::from(place_request.clone());
        
        assert_eq!(order.id, place_request.new_order_id);
        assert_eq!(order.account_id, place_request.account_id);
        assert_eq!(order.side, place_request.side);
        assert_eq!(order.order_type, place_request.order_type);
        assert_eq!(order.limit_price, place_request.limit_price);
        assert_eq!(order.base_amount, place_request.base_amount);
        assert_eq!(order.remaining_base, place_request.base_amount);
        assert_eq!(order.remaining_quote, 1000000000); // 10000 * 100000
        assert_eq!(order.filled_base, 0);
        assert_eq!(order.filled_quote, 0);
        assert_eq!(order.ext_id, place_request.ext_id);
    }
} 
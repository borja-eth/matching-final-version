use std::collections::HashMap;

use uuid::Uuid;

use crate::domain::services::orderbook::{
    OrderbookError,
    depth::PriceLevel as DepthLevel,
};

use super::types::{Order, Side, OrderStatus};

// Define clear event types for inter-thread communication
pub enum OrderbookEvent {
    NewOrder(Order),
    CancelOrder(Uuid),
    Snapshot,
    Halt,
    Resume,
}

#[derive(Debug)]
pub enum OrderbookResult {
    Add(AddOrderResult),
    Cancelled(CancelledOrderResult),
    Error(OrderbookError),
    Halted,
    Resumed,
    Snapshot(OrderbookSnapshot),
}

#[derive(Debug, Clone)]
pub struct OrderbookSnapshot {
    pub depth_levels: Vec<DepthLevel>,
}

// Helper type alias
pub type DepthKey = i64;

// Helper struct to represent the best bid and ask
#[derive(Debug, Clone)]
pub struct BestBidAndAsk {
    pub best_bid: Option<i64>,
    pub best_ask: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AddOrderResult {
    pub matches: Vec<Match>,
    pub rejected_orders: Vec<Order>,
    pub new_order: Option<Order>,
    pub depth_changes: HashMap<DepthKey, DepthLevel>,
    pub best_bid_and_ask: BestBidAndAsk,
}

#[derive(Debug, Clone)]
pub struct CancelledOrderResult {
    pub depth_changes: DepthLevel,
    pub order: Order,
    pub best_bid_and_ask: BestBidAndAsk,
}

#[derive(Debug, Clone)]
pub struct Match {
    pub taker_order_id: Uuid,
    pub maker_order_id: Uuid,
    pub taker_account_id: Uuid,
    pub maker_account_id: Uuid,
    pub maker_status: OrderStatus,
    pub taker_status: OrderStatus,
    pub match_base_amount: u64,
    pub match_quote_amount: u64,
    pub seq_num: u64,
    pub limit_price: i64,
    pub taker_side: Side,
}

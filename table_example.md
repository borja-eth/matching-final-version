//! Order Book Module
//!
//! This module implements the core order book functionality, managing price levels,
//! order storage, and book events. It provides efficient operations for adding,
//! canceling, and replacing orders, as well as querying book state.
//!
//! ```
//! +---------------------------------------------------------------------------------------+
//! |                               ORDER BOOK SUMMARY                                      |
//! +---------------------------------------------------------------------------------------+
//! | STRUCTS              | DESCRIPTION                                                    |
//! |---------------------------------------------------------------------------------------|
//! | PriceLevel           | Price level in the book:                                       |
//! |                      | - orders: Vec<Order> - Orders at this price                    |
//! |                      | - total_quantity: AtomicU32 - Total quantity at this price        |
//! |                      | - is_dirty: AtomicBool - Tracks if the level needs updating        |
//! | Book                 | Complete order book:                                           |
//! |                      | - symbol: String - Trading instrument                         |
//! |                      | - bids: Arc<SkipMap<u32, PriceLevel>> - Buy orders           |
//! |                      | - asks: Arc<SkipMap<u32, PriceLevel>> - Sell orders          |
//! |                      | - next_id: Arc<AtomicU64> - Next order ID                    |
//! |                      | - total_orders: Arc<AtomicU64> - Total orders processed      |
//! |                      | - total_quantity: Arc<AtomicU64> - Total quantity            |
//! |                      | - event_dispatcher: Option<EventDispatcher> - Event system   |
//! +---------------------------------------------------------------------------------------+
//! 
//! +---------------------------------------------------------------------------------------+
//! |                                  PUBLIC METHODS                                       |
//! +---------------------------------------------------------------------------------------+
//! | FUNCTION                 | MUTABILITY | DESCRIPTION                                |
//! |---------------------------------------------------------------------------------------|
//! | new                      | immut      | Creates new order book                     |
//! | new_with_dispatcher      | immut      | Creates book with event dispatcher         |
//! | set_event_dispatcher     | mut        | Sets event dispatcher                     |
//! | get_next_id              | immut      | Gets next available order ID              |
//! | symbol                   | immut      | Gets book's symbol                        |
//! | add_order                | immut      | Adds order to book                        |
//! | best_bid                 | immut      | Gets best bid price                       |
//! | best_ask                 | immut      | Gets best ask price                       |
//! | spread                   | immut      | Gets current spread                       |
//! | quantity_at_price        | immut      | Gets quantity at price level              |
//! | orders_at_price          | immut      | Gets number of orders at price            |
//! | cancel_order             | immut      | Cancels order from book                   |
//! | replace_order            | immut      | Replaces existing order                   |
//! | check_fok_liquidity      | immut      | Checks FOK order liquidity                |
//! +---------------------------------------------------------------------------------------+
//! 
//! +---------------------------------------------------------------------------------------+
//! |                                  PRIVATE METHODS                                      |
//! +---------------------------------------------------------------------------------------+
//! | FUNCTION                 | MUTABILITY | DESCRIPTION                                |
//! |---------------------------------------------------------------------------------------|
//! | check_and_emit_best_prices| immut     | Emits best price change events            |
//! +---------------------------------------------------------------------------------------+
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering};
use crossbeam_skiplist::SkipMap;
use crate::order::{Order, OrderType};
use super::events::{EventDispatcher, BookEvent, MarketDataEvent, PriceSource, OrderEvent};
use tracing::{info, error};

/// Represents a price level in the order book
//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements the core matching engine logic for processing orders and generating trades.
// The matching engine follows price-time priority to ensure fair order execution.
//
// NOTE ON DEAD CODE: Some specialized processing methods in this file are marked with
// #[allow(dead_code)] because they are called dynamically from process_order() through match
// expressions based on order types and time-in-force. The compiler cannot always detect this
// dynamic dispatch pattern, leading to false positives in dead code analysis.
// These methods are critical for performance optimization as they avoid excessive branching
// in the hot path.
//
// | Component                | Description                                                |
// |--------------------------|-----------------------------------------------------------|
// | MatchingEngine           | Main engine for processing and matching orders            |
// | TimeInForce              | Order duration policy (GTC, IOC)                          |
// | MatchResult              | Represents the outcome of a matching operation            |
// | MatchingError            | Error types specific to the matching process              |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | MatchingEngine          | Core matching engine                              | process_order    |
// |                         |                                                   | match_order      |
// |                         |                                                   | cancel_order     |
// |-------------------------|---------------------------------------------------|------------------|
// | MatchResult             | Result of a matching operation                    | trades           |
// |                         |                                                   | processed_order  |
// |                         |                                                   | affected_orders  |
//
//--------------------------------------------------------------------------------------------------
// ENUMS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Variants         |
// |-------------------------|---------------------------------------------------|------------------|
// | TimeInForce             | Order duration policy                             | GTC, IOC         |
// | MatchingError           | Errors that can occur during matching             | InvalidOrder     |
// |                         |                                                   | OrderNotFound    |
// |                         |                                                   | InsufficientLiq  |
//
//--------------------------------------------------------------------------------------------------
// FUNCTIONS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Return Type      |
// |-------------------------|---------------------------------------------------|------------------|
// | process_order           | Process a new order                               | Result<MatchResu>|
// | match_limit_order       | Match a limit order against the book              | Result<MatchResu>|
// | cancel_order            | Cancel an existing order                          | Result<Order>    |
//--------------------------------------------------------------------------------------------------

use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;
use chrono::Utc;

use crate::domain::services::orderbook::orderbook::OrderBook;
use crate::domain::models::types::{Order, Side, OrderType, OrderStatus, Trade, TimeInForce};
use crate::domain::services::orderbook::depth::DepthTracker;

/// Errors that can occur during the matching process.
#[derive(Error, Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum MatchingError {
    /// The order is invalid for processing (e.g., wrong status, missing required fields).
    #[error("Invalid order for processing: {0}")]
    InvalidOrder(String),
    
    /// The specified order was not found in the order book.
    #[error("Order with ID {0} not found")]
    OrderNotFound(Uuid),
    
    /// There is insufficient liquidity to fill a market order.
    #[error("Insufficient liquidity to fill market order")]
    InsufficientLiquidity,
    
    /// The order is for a different instrument than the engine is managing.
    #[error("Order instrument ID does not match engine")]
    WrongInstrument,
}

/// Type alias for Result with MatchingError
pub type MatchingResult<T> = Result<T, MatchingError>;

/// Represents the outcome of a matching operation.
#[derive(Debug, Clone, Default)]
pub struct MatchResult {
    /// Trades generated from the matching process
    pub trades: Vec<Trade>,
    
    /// The order after processing (may be filled, partially filled, or cancelled)
    pub processed_order: Option<Order>,
    
    /// Orders that were affected by this match (e.g., partially filled resting orders)
    pub affected_orders: Vec<Order>,
}

/// The core matching engine responsible for processing orders and generating trades.
/// 
/// # Overview
/// 
/// The matching engine is the central component of the trading system, responsible for:
/// 
/// * Processing incoming orders (limit, market, stop, etc.)
/// * Matching orders according to price-time priority
/// * Maintaining the order book
/// * Generating trades when orders match
/// * Tracking order book depth
/// 
/// # Price-Time Priority
/// 
/// Orders are matched according to strict price-time priority rules:
/// 
/// * Better prices are matched first (higher bids, lower asks)
/// * At the same price level, orders are matched in chronological order (FIFO)
/// 
/// # Order Types
/// 
/// The engine supports several order types:
/// 
/// * **Limit**: Orders with a specified price constraint
/// * **Market**: Orders to be executed immediately at the best available price
/// * **Stop**: Orders that become market orders when a trigger price is reached
/// * **StopLimit**: Orders that become limit orders when a trigger price is reached
/// 
/// # Time In Force
/// 
/// Orders can have different duration policies:
/// 
/// * **GTC (Good 'Til Cancelled)**: Orders remain active until explicitly cancelled
/// * **IOC (Immediate Or Cancel)**: Orders must be executed immediately or cancelled
#[derive(Debug)]
pub struct MatchingEngine {
    /// The order book for the instrument this engine is managing
    order_book: OrderBook,
    
    /// Maps order IDs to their location (side, price) for fast cancellation
    order_index: HashMap<Uuid, (Side, i64)>,
    
    /// Sequence counter for assigning order priorities
    next_sequence_id: u64,
    
    /// Instrument ID this engine is managing
    instrument_id: Uuid,

    /// Depth tracker for maintaining aggregated order book view
    depth_tracker: DepthTracker,
}

impl MatchingEngine {
    /// Creates a new matching engine for a specific instrument.
    ///
    /// This constructor creates a matching engine without event handlers,
    /// meaning it will not emit events when orders are processed or trades are executed.
    ///
    /// # Arguments
    ///
    /// * `instrument_id` - The unique identifier of the instrument this engine will manage
    ///
    /// # Returns
    ///
    /// A new `MatchingEngine` instance configured for the specified instrument
    ///
    /// # Examples
    ///
    /// ```
    /// use uuid::Uuid;
    /// use crate::domain::services::matching_engine::MatchingEngine;
    ///
    /// let instrument_id = Uuid::new_v4();
    /// let engine = MatchingEngine::new(instrument_id);
    /// ```
    #[inline]
    pub fn new(instrument_id: Uuid) -> Self {
        Self {
            order_book: OrderBook::new(instrument_id),
            order_index: HashMap::new(),
            next_sequence_id: 1,
            instrument_id,
            depth_tracker: DepthTracker::new(instrument_id),
        }
    }
    
    /// Processes a new order through the matching engine.
    ///
    /// This is the main entry point for order processing. The method will:
    /// 1. Assign a sequence ID to the order for price-time priority
    /// 2. Validate the order is for the correct instrument
    /// 3. Route the order to the appropriate specialized processor based on type and TIF
    ///
    /// # Arguments
    ///
    /// * `order` - The order to process
    /// * `time_in_force` - Duration policy for the order
    ///
    /// # Returns
    ///
    /// A `MatchResult` containing the processed order, any trades generated,
    /// and any other orders affected by the matching process
    ///
    /// # Errors
    ///
    /// Returns `MatchingError` if:
    /// * The order is for a different instrument (`WrongInstrument`)
    /// * The order is invalid for processing (`InvalidOrder`)
    /// * There is insufficient liquidity for a market order (`InsufficientLiquidity`)
    ///
    /// # Examples
    ///
    /// ```
    /// use uuid::Uuid;
    /// use crate::domain::models::types::{Order, TimeInForce};
    /// use crate::domain::services::matching_engine::MatchingEngine;
    ///
    /// let instrument_id = Uuid::new_v4();
    /// let mut engine = MatchingEngine::new(instrument_id);
    ///
    /// // Create an order (details omitted for brevity)
    /// let order = Order::new_limit_order(/* ... */);
    ///
    /// let result = engine.process_order(order, TimeInForce::GTC);
    /// ```
    pub fn process_order(&mut self, mut order: Order, time_in_force: TimeInForce) -> MatchingResult<MatchResult> {
        // Set a sequence ID for the order
        order.sequence_id = self.next_sequence_id;
        self.next_sequence_id += 1;
        
        // If order is for another instrument, reject it
        if order.instrument_id != self.instrument_id {
            return Err(MatchingError::WrongInstrument);
        }
        
        // Use specialized paths based on order type and time in force
        let result = match (order.order_type, time_in_force) {
            // Limit orders with GTC (most common case)
            (OrderType::Limit, TimeInForce::GTC) => {
                self.process_limit_gtc_order(order)
            },
            // Limit orders with IOC
            (OrderType::Limit, TimeInForce::IOC) => {
                self.process_limit_ioc_order(order)
            },
            // Market orders (always treated as IOC)
            (OrderType::Market, _) => {
                order.limit_price = None;
                self.process_market_order(order)
            },
            // Stop orders
            (OrderType::Stop, time_in_force) => {
                self.process_stop_order(order, time_in_force)
            },
            // StopLimit orders
            (OrderType::StopLimit, time_in_force) => {
                self.process_stop_limit_order(order, time_in_force)
            }
        };
        
        result
    }

    /// Specialized method for processing limit orders with GTC time in force.
    /// 
    /// This optimized path reduces branching and simplifies the logic specifically for
    /// limit orders with GTC time in force, which is the most common case.
    /// 
    /// # Order Processing Flow
    ///
    /// 1. Validates the limit price exists
    /// 2. Attempts to match the order against the opposite side of the book
    /// 3. If not fully filled, adds the remaining quantity to the book
    ///
    /// # Arguments
    ///
    /// * `order` - The GTC limit order to process
    ///
    /// # Returns
    ///
    /// A `MatchResult` containing the processed order and any trades generated
    ///
    /// # Errors
    ///
    /// Returns `InvalidOrder` if the limit price is missing
    #[inline]
    #[allow(dead_code)] // Called dynamically from process_order via match expression
    fn process_limit_gtc_order(&mut self, mut order: Order) -> MatchingResult<MatchResult> {
        // Validate limit price exists
        if order.limit_price.is_none() {
            return Err(MatchingError::InvalidOrder("Limit order must have a price".to_string()));
        }

        let mut result = self.match_order(&mut order)?;
        
        // For GTC orders that aren't fully filled, add to the book
        if order.status != OrderStatus::Filled {
            self.add_to_book(&order);
        }
        
        result.processed_order = Some(order);
        Ok(result)
    }
    
    /// Specialized method for processing limit orders with IOC time in force.
    /// 
    /// This optimized path reduces branching and simplifies the logic specifically for
    /// limit orders with IOC time in force. IOC orders must be executed immediately
    /// (fully or partially) or be cancelled.
    ///
    /// # Order Processing Flow
    ///
    /// 1. Validates the limit price exists
    /// 2. Attempts to match the order against the opposite side of the book
    /// 3. Any unfilled portion is cancelled (never added to the book)
    ///
    /// # Arguments
    ///
    /// * `order` - The IOC limit order to process
    ///
    /// # Returns
    ///
    /// A `MatchResult` containing the processed order and any trades generated
    ///
    /// # Errors
    ///
    /// Returns `InvalidOrder` if the limit price is missing
    #[inline]
    #[allow(dead_code)] // Called dynamically from process_order via match expression
    fn process_limit_ioc_order(&mut self, mut order: Order) -> MatchingResult<MatchResult> {
        // Validate limit price exists
        if order.limit_price.is_none() {
            return Err(MatchingError::InvalidOrder("Limit order must have a price".to_string()));
        }

        let mut result = self.match_order(&mut order)?;
        
        // For IOC orders that aren't fully filled, mark as cancelled
        if order.status != OrderStatus::Filled {
            if order.status == OrderStatus::Submitted {
                order.status = OrderStatus::Cancelled;
            } else {
                order.status = OrderStatus::PartiallyFilledCancelled;
            }
        }
        
        result.processed_order = Some(order);
        Ok(result)
    }
    
    /// Specialized method for processing market orders.
    /// 
    /// Market orders are always treated as IOC regardless of the specified time in force.
    /// This optimized path reduces branching and simplifies the logic specifically for
    /// market orders.
    ///
    /// # Order Processing Flow
    ///
    /// 1. Attempts to match the order against the opposite side of the book at any price
    /// 2. Any unfilled portion is cancelled (never added to the book)
    /// 3. If no liquidity is available, returns InsufficientLiquidity error
    ///
    /// # Arguments
    ///
    /// * `order` - The market order to process
    ///
    /// # Returns
    ///
    /// A `MatchResult` containing the processed order and any trades generated
    ///
    /// # Errors
    ///
    /// Returns `InsufficientLiquidity` if there are no matching orders in the book
    #[inline]
    #[allow(dead_code)] // Called dynamically from process_order via match expression
    fn process_market_order(&mut self, mut order: Order) -> MatchingResult<MatchResult> {
        let mut result = self.match_order(&mut order)?;
        
        // Market orders are always treated as IOC, so if not fully filled, mark as cancelled
        if order.status != OrderStatus::Filled {
            if order.status == OrderStatus::Submitted {
                order.status = OrderStatus::Cancelled;
            } else {
                order.status = OrderStatus::PartiallyFilledCancelled;
            }
        }
        
        result.processed_order = Some(order);
        Ok(result)
    }
    
    /// Specialized method for processing stop orders.
    /// 
    /// Stop orders wait for a price trigger before becoming market orders.
    /// They are stored in a waiting status until the market price reaches 
    /// the trigger level.
    ///
    /// # Order Processing Flow
    ///
    /// 1. Validates the trigger price exists
    /// 2. Sets the order to waiting trigger status
    /// 3. Returns the order without attempting to match it
    ///
    /// # Arguments
    ///
    /// * `order` - The stop order to process
    /// * `_time_in_force` - Will be applied when the stop is triggered
    ///
    /// # Returns
    ///
    /// A `MatchResult` containing the processed order (in waiting state)
    ///
    /// # Errors
    ///
    /// Returns `InvalidOrder` if the trigger price is missing
    #[inline]
    #[allow(dead_code)] // Called dynamically from process_order via match expression
    fn process_stop_order(&mut self, mut order: Order, _time_in_force: TimeInForce) -> MatchingResult<MatchResult> {
        // Validate trigger price exists
        if order.trigger_price.is_none() {
            return Err(MatchingError::InvalidOrder("Stop order must have a trigger price".to_string()));
        }
        
        // Set order to waiting trigger status
        order.status = OrderStatus::WaitingTrigger;
        
        // Since stop orders aren't immediately matched, just return the order
        // When the trigger condition is met, it will be converted to a market order
        // and processed through process_market_order
        let mut result = MatchResult::default();
        result.processed_order = Some(order);
        Ok(result)
    }
    
    /// Specialized method for processing stop limit orders.
    /// 
    /// Stop limit orders wait for a price trigger before becoming limit orders.
    /// They are stored in a waiting status until the market price reaches 
    /// the trigger level.
    ///
    /// # Order Processing Flow
    ///
    /// 1. Validates both trigger price and limit price exist
    /// 2. Sets the order to waiting trigger status
    /// 3. Returns the order without attempting to match it
    ///
    /// # Arguments
    ///
    /// * `order` - The stop limit order to process
    /// * `_time_in_force` - Will be applied when the stop is triggered
    ///
    /// # Returns
    ///
    /// A `MatchResult` containing the processed order (in waiting state)
    ///
    /// # Errors
    ///
    /// Returns `InvalidOrder` if either the trigger price or limit price is missing
    #[inline]
    #[allow(dead_code)] // Called dynamically from process_order via match expression
    fn process_stop_limit_order(&mut self, mut order: Order, _time_in_force: TimeInForce) -> MatchingResult<MatchResult> {
        // Validate trigger price and limit price exist
        if order.trigger_price.is_none() {
            return Err(MatchingError::InvalidOrder("Stop limit order must have a trigger price".to_string()));
        }
        
        if order.limit_price.is_none() {
            return Err(MatchingError::InvalidOrder("Stop limit order must have a limit price".to_string()));
        }
        
        // Set order to waiting trigger status
        order.status = OrderStatus::WaitingTrigger;
        
        // Since stop limit orders aren't immediately matched, just return the order
        // When the trigger condition is met, it will be converted to a limit order
        // and processed through process_limit_gtc_order or process_limit_ioc_order
        let mut result = MatchResult::default();
        result.processed_order = Some(order);
        Ok(result)
    }
    
    /// Matches an order against the order book.
    ///
    /// This is the core matching algorithm that implements price-time priority
    /// matching. It continues to match against the best opposing orders until
    /// the order is fully filled or no more matching is possible.
    ///
    /// # Performance Optimizations
    ///
    /// This method includes several optimizations:
    /// * Pre-extraction of fields to avoid repeated lookups
    /// * Reuse of trade objects to reduce allocations
    /// * Batched depth tracker and index updates
    /// * Early exit checks for common cases
    ///
    /// # Arguments
    ///
    /// * `order` - Mutable reference to the order to match
    ///
    /// # Returns
    ///
    /// A `MatchResult` containing the trades generated and affected orders
    ///
    /// # Errors
    ///
    /// Returns `InvalidOrder` if a limit order has no price
    /// Returns `OrderNotFound` if an opposing order disappears during matching
    /// Returns `InsufficientLiquidity` for market orders with no matches
    #[inline(always)]
    fn match_order(&mut self, order: &mut Order) -> MatchingResult<MatchResult> {
        let mut result = MatchResult::default();
        
        // Find the opposite side - this only needs to be done once
        let opposite_side = match order.side {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        };
        
        // Pre-extract the limit price for efficiency if this is a limit order
        let is_limit_order = order.order_type == OrderType::Limit;
        let limit_price = if is_limit_order {
            match order.limit_price {
                Some(price) => price,
                None => return Err(MatchingError::InvalidOrder("Limit order must have a price".to_string())),
            }
        } else {
            0 // Dummy value for market orders, won't be used
        };
        
        // Pre-allocate trades vector to reduce allocations in the hot path
        result.trades.reserve(8);
        result.affected_orders.reserve(8);
        
        // OPTIMIZATION 1: Reuse Trade object to reduce allocations
        let mut trade = Trade {
            id: Uuid::nil(),
            instrument_id: self.instrument_id,
            maker_order_id: Uuid::nil(),
            taker_order_id: order.id,
            base_amount: 0,
            quote_amount: 0,
            price: 0,
            created_at: Utc::now(),
        };
        
        // OPTIMIZATION 2: Batch depth tracker and index updates
        let mut removals = Vec::with_capacity(8);
        let mut additions = Vec::with_capacity(8);
        
        let mut remaining_base = order.remaining_base;
        let is_market_order = order.order_type == OrderType::Market;
        
        // Early check for filled status
        if remaining_base == 0 {
            order.status = OrderStatus::Filled;
            return Ok(result);
        }
        
        // Keep matching until the order is filled or no more matches are possible
        loop {
            // Get the best opposing order - only do this lookup once per iteration
            let best_opposing_order = match opposite_side {
                Side::Bid => self.order_book.get_best_bid(),
                Side::Ask => self.order_book.get_best_ask(),
            };
            
            // Check if there's no matching order - early exit
            let best_opposing_order = match best_opposing_order {
                Some(order_ref) => order_ref,
                None => break,
            };
            
            // Extract all needed fields from the opposing order once
            let opposing_id = best_opposing_order.id;
            let opposing_price = match best_opposing_order.limit_price {
                Some(price) => price,
                None => return Err(MatchingError::InvalidOrder("Opposing order must have a price".to_string())),
            };
            
            // For limit orders, check if the price is acceptable
            if is_limit_order {
                // Use the cached limit price
                let price_acceptable = match order.side {
                    Side::Bid => opposing_price <= limit_price, // Buy: best ask <= my bid
                    Side::Ask => opposing_price >= limit_price, // Sell: best bid >= my ask
                };
                
                if !price_acceptable {
                    break;
                }
            }
            
            // Remove the order from the book - we've already checked everything we need
            let mut opposing_order = match self.order_book.remove_order(opposing_id) {
                Ok(order) => order,
                Err(_) => return Err(MatchingError::OrderNotFound(opposing_id)),
            };
            
            // Add to removals for batch processing
            removals.push(opposing_order.clone());
            
            // Calculate matched quantity
            let matched_qty = std::cmp::min(remaining_base, opposing_order.remaining_base);
            
            // Calculate quote amount using the previously obtained opposing price
            let quote_amount = matched_qty * (opposing_price as u64);
            
            // Update trade object without creating a new one
            trade.id = Uuid::new_v4();
            trade.maker_order_id = opposing_id;
            trade.base_amount = matched_qty;
            trade.quote_amount = quote_amount;
            trade.price = opposing_price;
            trade.created_at = Utc::now();
            
            // Update order states
            remaining_base -= matched_qty;
            order.filled_base += matched_qty;
            order.filled_quote += quote_amount;
            
            opposing_order.remaining_base -= matched_qty;
            opposing_order.filled_base += matched_qty;
            opposing_order.filled_quote += quote_amount;
            
            // Update order statuses
            if order.status == OrderStatus::Submitted && remaining_base != 0 {
                order.status = OrderStatus::PartiallyFilled;
            }
            
            if opposing_order.remaining_base == 0 {
                opposing_order.status = OrderStatus::Filled;
            } else {
                opposing_order.status = OrderStatus::PartiallyFilled;
                additions.push(opposing_order.clone());
            }
            
            // Record trade and affected order
            result.trades.push(trade.clone());
            result.affected_orders.push(opposing_order);
            
            // Exit if order is fully filled
            if remaining_base == 0 {
                order.status = OrderStatus::Filled;
                break;
            }
        }
        
        // Update the order's remaining base amount
        order.remaining_base = remaining_base;
        
        // Process batched updates - only if there were any matches
        if !removals.is_empty() || !additions.is_empty() {
            // Remove from index and update depth tracker for removed orders
            for removed_order in &removals {
                self.order_index.remove(&removed_order.id);
                self.depth_tracker.update_order_removed(removed_order);
            }
            
            // Add to book, update index and depth tracker for added orders
            for added_order in &additions {
                let _ = self.order_book.add_order(added_order.clone());
                if let Some(price) = added_order.limit_price {
                    self.order_index.insert(added_order.id, (added_order.side, price));
                    self.depth_tracker.update_order_added(added_order);
                }
            }
        }
        
        // For market orders with no matches, return an error
        if is_market_order && order.status == OrderStatus::Submitted && result.trades.is_empty() {
            return Err(MatchingError::InsufficientLiquidity);
        }
        
        Ok(result)
    }
    
    /// Adds an order to the book and updates the index.
    /// 
    /// This method is used in the critical matching path, so it's inlined for performance.
    /// It updates both the order book and the depth tracker.
    ///
    /// # Arguments
    ///
    /// * `order` - The order to add to the book
    #[inline]
    fn add_to_book(&mut self, order: &Order) {
        if let Some(price) = order.limit_price {
            let _ = self.order_book.add_order(order.clone());
            self.order_index.insert(order.id, (order.side, price));
            // Update depth tracker
            self.depth_tracker.update_order_added(order);
        }
    }
    
    /// Cancels an existing order in the order book.
    ///
    /// # Order Cancellation Flow
    ///
    /// 1. Looks up the order location in the index
    /// 2. Removes the order from the book
    /// 3. Updates the depth tracker
    /// 4. Updates the order status to cancelled or partially filled cancelled
    ///
    /// # Arguments
    ///
    /// * `order_id` - The ID of the order to cancel
    ///
    /// # Returns
    ///
    /// The cancelled order if found
    ///
    /// # Errors
    ///
    /// Returns `OrderNotFound` if the order is not in the book
    #[inline]
    pub fn cancel_order(&mut self, order_id: Uuid) -> MatchingResult<Order> {
        // Look up the order location in our index
        if let Some((_side, _price)) = self.order_index.remove(&order_id) {
            if let Ok(mut order) = self.order_book.remove_order(order_id) {
                // Update depth tracker
                self.depth_tracker.update_order_removed(&order);
                
                // Update order status
                if order.status == OrderStatus::Submitted {
                    order.status = OrderStatus::Cancelled;
                } else {
                    order.status = OrderStatus::PartiallyFilledCancelled;
                }
                
                return Ok(order);
            }
        }
        
        Err(MatchingError::OrderNotFound(order_id))
    }
    
    /// Gets the current state of the order book.
    ///
    /// # Returns
    ///
    /// A reference to the current order book
    pub fn order_book(&self) -> &OrderBook {
        &self.order_book
    }
    
    /// Gets the instrument ID this engine is managing.
    ///
    /// # Returns
    ///
    /// The UUID of the instrument this engine is responsible for
    pub fn instrument_id(&self) -> Uuid {
        self.instrument_id
    }
    
    /// Gets a snapshot of the current order book depth
    ///
    /// This method provides an aggregated view of the order book with
    /// volume information at each price level.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of price levels to include per side
    ///
    /// # Returns
    ///
    /// A snapshot of the current depth state
    pub fn get_depth(&mut self, limit: usize) -> crate::domain::services::orderbook::depth::DepthSnapshot {
        self.depth_tracker.get_snapshot(limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::types::CreatedFrom;
    
    /// Creates a test order with the specified parameters.
    ///
    /// This helper function standardizes order creation across tests and
    /// scales the quantities and prices to match the expected decimal places.
    ///
    /// # Arguments
    ///
    /// * `side` - Order side (bid/ask)
    /// * `order_type` - Type of order (limit, market, etc.)
    /// * `price` - Optional price in base units (will be scaled by 1000)
    /// * `quantity` - Quantity in base units (will be scaled by 1000)
    /// * `instrument_id` - ID of the instrument
    ///
    /// # Returns
    ///
    /// A new Order instance configured with the specified parameters
    fn create_test_order(
        side: Side, 
        order_type: OrderType, 
        price: Option<i64>, 
        quantity: u64,
        instrument_id: Uuid
    ) -> Order {
        let now = Utc::now();
        let remaining_quote = match price {
            Some(p) => (p as u64 * quantity) * 1000, // Scale by 1000 to match expected decimal places
            None => 0,
        };
        
        Order {
            id: Uuid::new_v4(),
            ext_id: Some("test-order".to_string()),
            account_id: Uuid::new_v4(),
            order_type,
            instrument_id,
            side,
            limit_price: price.map(|p| p * 1000), // Scale price by 1000
            trigger_price: None,
            base_amount: quantity * 1000, // Scale base amount by 1000
            remaining_base: quantity * 1000, // Scale remaining base by 1000
            filled_quote: 0,
            filled_base: 0,
            remaining_quote,
            expiration_date: now + chrono::Duration::days(365),
            status: OrderStatus::Submitted,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 0, // Will be set by engine
            time_in_force: TimeInForce::GTC,
        }
    }
    
    /// Tests that the specialized process_limit_gtc_order method works correctly.
    ///
    /// This test verifies that:
    /// 1. A limit GTC order is properly validated
    /// 2. The order is added to the book if not matched
    /// 3. The correct match result is returned
    #[test]
    fn test_process_limit_gtc_order_directly() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create a limit order
        let order = create_test_order(
            Side::Bid,
            OrderType::Limit,
            Some(100),
            1,
            instrument_id
        );
        
        // Test the method directly
        let result = engine.process_limit_gtc_order(order);
        assert!(result.is_ok());
        
        let match_result = result.unwrap();
        assert_eq!(match_result.trades.len(), 0);
        assert!(match_result.processed_order.is_some());
    }
    
    /// Tests that the specialized process_limit_ioc_order method works correctly.
    ///
    /// This test verifies that:
    /// 1. A limit IOC order is properly validated
    /// 2. The order is matched against existing orders in the book
    /// 3. The order is filled and the correct match result is returned
    #[test]
    fn test_process_limit_ioc_order_directly() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a resting order first
        let resting_order = create_test_order(
            Side::Ask,
            OrderType::Limit,
            Some(100),
            1,
            instrument_id
        );
        engine.process_order(resting_order, TimeInForce::GTC).unwrap();
        
        // Create a matching IOC order
        let ioc_order = create_test_order(
            Side::Bid,
            OrderType::Limit,
            Some(100),
            1,
            instrument_id
        );
        
        // Test the method directly
        let result = engine.process_limit_ioc_order(ioc_order);
        assert!(result.is_ok());
        
        let match_result = result.unwrap();
        assert_eq!(match_result.trades.len(), 1); // Should have matched
        assert!(match_result.processed_order.is_some());
        assert_eq!(match_result.processed_order.unwrap().status, OrderStatus::Filled);
    }
    
    /// Tests that the specialized process_market_order method works correctly.
    ///
    /// This test verifies that:
    /// 1. A market order is matched against existing orders in the book
    /// 2. The order is filled and the correct match result is returned
    #[test]
    fn test_process_market_order_directly() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a resting order first
        let resting_order = create_test_order(
            Side::Ask,
            OrderType::Limit,
            Some(100),
            1,
            instrument_id
        );
        engine.process_order(resting_order, TimeInForce::GTC).unwrap();
        
        // Create a market order
        let market_order = create_test_order(
            Side::Bid,
            OrderType::Market,
            None,
            1,
            instrument_id
        );
        
        // Test the method directly
        let result = engine.process_market_order(market_order);
        assert!(result.is_ok());
        
        let match_result = result.unwrap();
        assert_eq!(match_result.trades.len(), 1); // Should have matched
        assert!(match_result.processed_order.is_some());
        assert_eq!(match_result.processed_order.unwrap().status, OrderStatus::Filled);
    }
    
    /// Tests that the specialized process_stop_order method works correctly.
    ///
    /// This test verifies that:
    /// 1. A stop order with a trigger price is properly validated
    /// 2. The order is set to waiting trigger status
    /// 3. No trades are generated at this stage
    #[test]
    fn test_process_stop_order_directly() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create a stop order
        let mut stop_order = create_test_order(
            Side::Bid,
            OrderType::Stop,
            None,
            1,
            instrument_id
        );
        stop_order.trigger_price = Some(100);
        
        // Test the method directly
        let result = engine.process_stop_order(stop_order, TimeInForce::GTC);
        assert!(result.is_ok());
        
        let match_result = result.unwrap();
        assert_eq!(match_result.trades.len(), 0); // No trades yet, waiting for trigger
        assert!(match_result.processed_order.is_some());
        assert_eq!(match_result.processed_order.unwrap().status, OrderStatus::WaitingTrigger);
    }
    
    /// Tests that the specialized process_stop_limit_order method works correctly.
    ///
    /// This test verifies that:
    /// 1. A stop limit order with both trigger and limit prices is properly validated
    /// 2. The order is set to waiting trigger status
    /// 3. No trades are generated at this stage
    #[test]
    fn test_process_stop_limit_order_directly() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create a stop limit order
        let mut stop_limit_order = create_test_order(
            Side::Bid,
            OrderType::StopLimit,
            Some(99),
            1,
            instrument_id
        );
        stop_limit_order.trigger_price = Some(100);
        
        // Test the method directly
        let result = engine.process_stop_limit_order(stop_limit_order, TimeInForce::GTC);
        assert!(result.is_ok());
        
        let match_result = result.unwrap();
        assert_eq!(match_result.trades.len(), 0); // No trades yet, waiting for trigger
        assert!(match_result.processed_order.is_some());
        assert_eq!(match_result.processed_order.unwrap().status, OrderStatus::WaitingTrigger);
    }
    
    /// Tests that the specialized match_order method works correctly.
    ///
    /// This test verifies that:
    /// 1. An order can be directly matched against the book
    /// 2. Trades are generated correctly
    /// 3. The order status is updated appropriately
    #[test]
    fn test_match_order_directly() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a resting order first
        let resting_order = create_test_order(
            Side::Ask,
            OrderType::Limit,
            Some(100),
            1,
            instrument_id
        );
        engine.process_order(resting_order, TimeInForce::GTC).unwrap();
        
        // Create an order to match
        let mut matching_order = create_test_order(
            Side::Bid,
            OrderType::Limit,
            Some(100),
            1,
            instrument_id
        );
        
        // Test the match_order method directly
        let result = engine.match_order(&mut matching_order);
        assert!(result.is_ok());
        
        let match_result = result.unwrap();
        assert_eq!(match_result.trades.len(), 1);
        assert_eq!(matching_order.status, OrderStatus::Filled);
    }
    
    /// Tests matching limit orders with GTC time in force.
    ///
    /// This test verifies that:
    /// 1. A limit buy order can be added to the book
    /// 2. A matching sell order will execute against it
    /// 3. Trades are generated with the correct details
    #[test]
    fn test_match_limit_orders_gtc() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(100), 
            1,
            instrument_id
        );
        
        let result = match engine.process_order(buy_order, TimeInForce::GTC) {
            Ok(result) => result,
            Err(e) => panic!("Failed to process order: {:?}", e),
        };
        assert!(result.trades.is_empty());
        
        let processed_order = match result.processed_order {
            Some(order) => order,
            None => panic!("Expected processed order to be present"),
        };
        assert_eq!(processed_order.status, OrderStatus::Submitted);
        
        // Add a matching GTC sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(100), 
            1,
            instrument_id
        );
        
        let result = match engine.process_order(sell_order, TimeInForce::GTC) {
            Ok(result) => result,
            Err(e) => panic!("Failed to process order: {:?}", e),
        };
        assert_eq!(result.trades.len(), 1);
        
        let processed_order = match result.processed_order {
            Some(order) => order,
            None => panic!("Expected processed order to be present"),
        };
        assert_eq!(processed_order.status, OrderStatus::Filled);
    }
    
    /// Tests matching limit orders with IOC time in force.
    ///
    /// This test verifies that:
    /// 1. A GTC limit order can be added to the book
    /// 2. A matching IOC order will execute against it
    /// 3. The IOC order is filled and not added to the book
    #[test]
    fn test_match_limit_orders_ioc() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(100), 
            1,
            instrument_id
        );
        
        match engine.process_order(buy_order, TimeInForce::GTC) {
            Ok(_) => {},
            Err(e) => panic!("Failed to process order: {:?}", e),
        };
        
        // Add a matching IOC sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(100), 
            1,
            instrument_id
        );
        
        let result = match engine.process_order(sell_order, TimeInForce::IOC) {
            Ok(result) => result,
            Err(e) => panic!("Failed to process order: {:?}", e),
        };
        assert_eq!(result.trades.len(), 1);
        
        let processed_order = match result.processed_order {
            Some(order) => order,
            None => panic!("Expected processed order to be present"),
        };
        assert_eq!(processed_order.status, OrderStatus::Filled);
    }
    
    /// Tests behavior of IOC orders that can't be fully filled.
    ///
    /// This test verifies that:
    /// 1. An IOC order matches what it can
    /// 2. Any unfilled portion is cancelled
    /// 3. The IOC order is not added to the book
    #[test]
    fn test_ioc_not_fully_filled() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(100), 
            1,
            instrument_id
        );
        
        match engine.process_order(buy_order, TimeInForce::GTC) {
            Ok(_) => {},
            Err(e) => panic!("Failed to process order: {:?}", e),
        };
        
        // Add a matching IOC sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(100), 
            1,
            instrument_id
        );
        
        let result = match engine.process_order(sell_order, TimeInForce::IOC) {
            Ok(result) => result,
            Err(e) => panic!("Failed to process order: {:?}", e),
        };
        assert_eq!(result.trades.len(), 1);
        
        let processed_order = match result.processed_order {
            Some(order) => order,
            None => panic!("Expected processed order to be present"),
        };
        assert_eq!(processed_order.status, OrderStatus::Filled);
        
        // Make sure the IOC order didn't get added to the book
        assert!(engine.order_book.get_best_ask().is_none());
    }
    
    /// Tests market order execution.
    ///
    /// This test verifies that:
    /// 1. A market order matches against the best available price
    /// 2. The order is fully filled at the resting order's price
    /// 3. The market order is never added to the book
    #[test]
    fn test_market_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(100), 
            1,
            instrument_id
        );
        
        engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Add a market buy order (treated as IOC)
        let market_order = create_test_order(
            Side::Bid, 
            OrderType::Market, 
            None, 
            1,
            instrument_id
        );
        
        let result = engine.process_order(market_order, TimeInForce::GTC).unwrap();
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.processed_order.unwrap().status, OrderStatus::Filled);
        assert_eq!(result.trades[0].price, 100000);
    }
    
    /// Tests market orders with no available liquidity.
    ///
    /// This test verifies that:
    /// 1. A market order with no matching liquidity is rejected
    /// 2. The correct error type is returned (InsufficientLiquidity)
    #[test]
    fn test_market_order_insufficient_liquidity() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a market buy order with no liquidity
        let market_order = create_test_order(
            Side::Bid, 
            OrderType::Market, 
            None, 
            1,
            instrument_id
        );
        
        let result = engine.process_order(market_order, TimeInForce::GTC);
        assert!(matches!(result, Err(MatchingError::InsufficientLiquidity)));
    }
    
    /// Tests order cancellation.
    ///
    /// This test verifies that:
    /// 1. An order can be cancelled by its ID
    /// 2. The cancelled order has the correct status
    /// 3. The order is removed from the book
    #[test]
    fn test_cancel_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(100), 
            1,
            instrument_id
        );
        
        let result = match engine.process_order(buy_order, TimeInForce::GTC) {
            Ok(result) => result,
            Err(e) => panic!("Failed to process order: {:?}", e),
        };
        
        let processed_order = match result.processed_order {
            Some(order) => order,
            None => panic!("Expected processed order to be present"),
        };
        let order_id = processed_order.id;
        
        // Cancel the order
        let cancelled = match engine.cancel_order(order_id) {
            Ok(order) => order,
            Err(e) => panic!("Failed to cancel order: {:?}", e),
        };
        assert_eq!(cancelled.status, OrderStatus::Cancelled);
        
        // Verify it's gone from the book
        assert!(engine.order_book.get_best_bid().is_none());
    }
}

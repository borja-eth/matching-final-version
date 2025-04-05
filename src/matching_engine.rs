//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements the core matching engine logic for processing orders and generating trades.
// The matching engine follows price-time priority to ensure fair order execution.
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
use rust_decimal::Decimal;
use thiserror::Error;
use uuid::Uuid;
use chrono::Utc;

use crate::orderbook::OrderBook;
use crate::types::{Order, Side, OrderType, OrderStatus, Trade, TimeInForce};
use crate::depth::DepthTracker;
use crate::events::{EventBus, MatchingEngineEvent};

/// Errors that can occur during the matching process.
#[derive(Error, Debug, Clone, PartialEq)]
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
#[derive(Debug)]
pub struct MatchingEngine {
    /// The order book for the instrument this engine is managing
    order_book: OrderBook,
    
    /// Maps order IDs to their location (side, price) for fast cancellation
    order_index: HashMap<Uuid, (Side, Decimal)>,
    
    /// Sequence counter for assigning order priorities
    next_sequence_id: u64,
    
    /// Instrument ID this engine is managing
    instrument_id: Uuid,

    /// Depth tracker for maintaining aggregated order book view
    depth_tracker: DepthTracker,
    
    /// Event bus for emitting events (optional)
    event_bus: Option<EventBus>,
}

impl MatchingEngine {
    /// Creates a new matching engine for a specific instrument.
    #[inline]
    pub fn new(instrument_id: Uuid) -> Self {
        Self {
            order_book: OrderBook::new(instrument_id),
            order_index: HashMap::new(),
            next_sequence_id: 1,
            instrument_id,
            depth_tracker: DepthTracker::new(instrument_id),
            event_bus: None,
        }
    }
    
    /// Creates a new matching engine with an event bus.
    pub fn with_event_bus(instrument_id: Uuid, event_bus: EventBus) -> Self {
        Self {
            order_book: OrderBook::new(instrument_id),
            order_index: HashMap::new(),
            next_sequence_id: 1,
            instrument_id,
            depth_tracker: DepthTracker::new(instrument_id),
            event_bus: Some(event_bus),
        }
    }
    
    /// Processes a new order through the matching engine.
    ///
    /// # Arguments
    /// * `order` - The order to process
    /// * `time_in_force` - Duration policy for the order
    ///
    /// # Returns
    /// A `MatchResult` containing the processed order and any trades generated
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
        
        // Emit events based on the result
        if let Ok(ref match_result) = result {
            self.emit_events_for_match_result(match_result);
        }
        
        result
    }

    /// Specialized method for processing limit orders with GTC time in force.
    /// 
    /// This optimized path reduces branching and simplifies the logic specifically for
    /// limit orders with GTC time in force, which is the most common case.
    #[inline]
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
    /// limit orders with IOC time in force.
    #[inline]
    fn process_limit_ioc_order(&mut self, mut order: Order) -> MatchingResult<MatchResult> {
        // Validate limit price exists
        if order.limit_price.is_none() {
            return Err(MatchingError::InvalidOrder("Limit order must have a price".to_string()));
        }

        let mut result = self.match_order(&mut order)?;
        
        // For IOC orders that aren't fully filled, mark as cancelled
        if order.status != OrderStatus::Filled {
            if order.status == OrderStatus::New {
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
    #[inline]
    fn process_market_order(&mut self, mut order: Order) -> MatchingResult<MatchResult> {
        let mut result = self.match_order(&mut order)?;
        
        // Market orders are always treated as IOC, so if not fully filled, mark as cancelled
        if order.status != OrderStatus::Filled {
            if order.status == OrderStatus::New {
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
    #[inline]
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
    #[inline]
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
    /// # Arguments
    /// * `order` - The order to match
    ///
    /// # Returns
    /// A `MatchResult` containing the trades generated
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
            Decimal::ZERO // Dummy value for market orders, won't be used
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
            base_amount: Decimal::ZERO,
            quote_amount: Decimal::ZERO,
            price: Decimal::ZERO,
            created_at: Utc::now(),
        };
        
        // OPTIMIZATION 2: Batch depth tracker and index updates
        let mut removals = Vec::with_capacity(8);
        let mut additions = Vec::with_capacity(8);
        
        let mut remaining_base = order.remaining_base;
        let is_market_order = order.order_type == OrderType::Market;
        
        // Early check for filled status
        if remaining_base.is_zero() {
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
            let mut opposing_order = match self.order_book.remove_order(
                opposing_id,
                opposite_side,
                opposing_price
            ) {
                Some(order) => order,
                None => return Err(MatchingError::OrderNotFound(opposing_id)),
            };
            
            // Add to removals for batch processing
            removals.push(opposing_order.clone());
            
            // Calculate matched quantity
            let matched_qty = Decimal::min(remaining_base, opposing_order.remaining_base);
            
            // Calculate quote amount using the previously obtained opposing price
            let quote_amount = matched_qty * opposing_price;
            
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
            if order.status == OrderStatus::New && !remaining_base.is_zero() {
                order.status = OrderStatus::PartiallyFilled;
            }
            
            if opposing_order.remaining_base.is_zero() {
                opposing_order.status = OrderStatus::Filled;
            } else {
                opposing_order.status = OrderStatus::PartiallyFilled;
                additions.push(opposing_order.clone());
            }
            
            // Record trade and affected order
            result.trades.push(trade.clone());
            result.affected_orders.push(opposing_order);
            
            // Exit if order is fully filled
            if remaining_base.is_zero() {
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
                self.order_book.add_order(added_order.clone());
                if let Some(price) = added_order.limit_price {
                    self.order_index.insert(added_order.id, (added_order.side, price));
                    self.depth_tracker.update_order_added(added_order);
                }
            }
        }
        
        // For market orders with no matches, return an error
        if is_market_order && order.status == OrderStatus::New && result.trades.is_empty() {
            return Err(MatchingError::InsufficientLiquidity);
        }
        
        Ok(result)
    }
    
    /// Adds an order to the book and updates the index.
    /// 
    /// This method is used in the critical matching path, so it's inlined for performance.
    #[inline]
    fn add_to_book(&mut self, order: &Order) {
        if let Some(price) = order.limit_price {
            self.order_book.add_order(order.clone());
            self.order_index.insert(order.id, (order.side, price));
            // Update depth tracker
            self.depth_tracker.update_order_added(order);
        }
    }
    
    /// Cancels an existing order in the order book.
    ///
    /// # Arguments
    /// * `order_id` - The ID of the order to cancel
    ///
    /// # Returns
    /// The cancelled order if found
    #[inline]
    pub fn cancel_order(&mut self, order_id: Uuid) -> MatchingResult<Order> {
        // Look up the order location in our index
        if let Some((side, price)) = self.order_index.remove(&order_id) {
            if let Some(mut order) = self.order_book.remove_order(order_id, side, price) {
                // Update depth tracker
                self.depth_tracker.update_order_removed(&order);
                
                // Update order status
                if order.status == OrderStatus::PartiallyFilled {
                    order.status = OrderStatus::PartiallyFilledCancelled;
                } else {
                    order.status = OrderStatus::Cancelled;
                }
                
                // Emit cancel event
                if let Some(ref event_bus) = self.event_bus {
                    let cancel_event = MatchingEngineEvent::OrderCancelled {
                        order: order.clone(),
                        timestamp: Utc::now(),
                    };
                    
                    let _ = event_bus.publish(cancel_event);
                }
                
                return Ok(order);
            }
        }
        
        Err(MatchingError::OrderNotFound(order_id))
    }
    
    /// Gets the current state of the order book.
    pub fn order_book(&self) -> &OrderBook {
        &self.order_book
    }
    
    /// Gets the instrument ID this engine is managing.
    pub fn instrument_id(&self) -> Uuid {
        self.instrument_id
    }

    /// Emits events based on the matching result
    fn emit_events_for_match_result(&self, result: &MatchResult) {
        if let Some(ref event_bus) = self.event_bus {
            // Emit events for trades
            for trade in &result.trades {
                let trade_event = MatchingEngineEvent::TradeExecuted {
                    trade: trade.clone(),
                    timestamp: Utc::now(),
                };
                
                let _ = event_bus.publish(trade_event);
            }
            
            // Emit event for the processed order
            if let Some(ref order) = result.processed_order {
                let order_event = match order.status {
                    OrderStatus::New => MatchingEngineEvent::OrderAdded {
                        order: order.clone(),
                        timestamp: Utc::now(),
                    },
                    OrderStatus::PartiallyFilled | OrderStatus::Filled => {
                        MatchingEngineEvent::OrderMatched {
                            order: order.clone(),
                            matched_quantity: order.filled_base,
                            timestamp: Utc::now(),
                        }
                    },
                    OrderStatus::Cancelled | OrderStatus::PartiallyFilledCancelled => {
                        MatchingEngineEvent::OrderCancelled {
                            order: order.clone(),
                            timestamp: Utc::now(),
                        }
                    },
                    _ => return, // No event for other statuses
                };
                
                let _ = event_bus.publish(order_event);
            }
            
            // Emit events for affected orders
            for order in &result.affected_orders {
                let affected_event = MatchingEngineEvent::OrderMatched {
                    order: order.clone(),
                    matched_quantity: order.filled_base,
                    timestamp: Utc::now(),
                };
                
                let _ = event_bus.publish(affected_event);
            }
            
            // We can't mutably borrow depth_tracker here, so we'll skip the depth event
            // The depth event will still be emitted when get_depth() is called
        }
    }
    
    /// Gets a depth snapshot from the order book
    ///
    /// # Arguments
    /// * `limit` - The maximum number of price levels to include per side
    ///
    /// # Returns
    /// A snapshot of the current order book depth
    pub fn get_depth(&mut self, limit: usize) -> crate::depth::DepthSnapshot {
        let snapshot = self.depth_tracker.get_snapshot(limit);
        
        // Emit depth update event
        if let Some(ref event_bus) = self.event_bus {
            let depth_event = MatchingEngineEvent::DepthUpdated {
                depth: snapshot.clone(),
                timestamp: Utc::now(),
            };
            
            let _ = event_bus.publish(depth_event);
        }
        
        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use crate::types::CreatedFrom;
    
    // Helper function to create test orders
    fn create_test_order(
        side: Side, 
        order_type: OrderType, 
        price: Option<Decimal>, 
        quantity: Decimal,
        instrument_id: Uuid
    ) -> Order {
        let now = Utc::now();
        let remaining_quote = match price {
            Some(p) => p * quantity,
            None => dec!(0),
        };
        
        Order {
            id: Uuid::new_v4(),
            ext_id: Some("test-order".to_string()),
            account_id: Uuid::new_v4(),
            order_type,
            instrument_id,
            side,
            limit_price: price,
            trigger_price: None,
            base_amount: quantity,
            remaining_base: quantity,
            filled_quote: dec!(0.0),
            filled_base: dec!(0.0),
            remaining_quote,
            expiration_date: now + chrono::Duration::days(365),
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 0, // Will be set by engine
        }
    }
    
    #[test]
    fn test_match_limit_orders_gtc() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
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
        assert_eq!(processed_order.status, OrderStatus::New);
        
        // Add a matching GTC sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
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
    
    #[test]
    fn test_match_limit_orders_ioc() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
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
            Some(dec!(100.0)), 
            dec!(1.0),
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
    
    #[test]
    fn test_ioc_not_fully_filled() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
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
            Some(dec!(100.0)), 
            dec!(1.0),
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
    
    #[test]
    fn test_market_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Add a market buy order (treated as IOC)
        let market_order = create_test_order(
            Side::Bid, 
            OrderType::Market, 
            None, 
            dec!(1.0),
            instrument_id
        );
        
        let result = engine.process_order(market_order, TimeInForce::GTC).unwrap();
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.processed_order.unwrap().status, OrderStatus::Filled);
        assert_eq!(result.trades[0].price, dec!(100.0));
    }
    
    #[test]
    fn test_market_order_insufficient_liquidity() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a market buy order with no liquidity
        let market_order = create_test_order(
            Side::Bid, 
            OrderType::Market, 
            None, 
            dec!(1.0),
            instrument_id
        );
        
        let result = engine.process_order(market_order, TimeInForce::GTC);
        assert!(matches!(result, Err(MatchingError::InsufficientLiquidity)));
    }
    
    #[test]
    fn test_cancel_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
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
    
    // === NEW EDGE CASE TESTS ===
    
    #[test]
    fn test_partial_fill() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(2.0), // Order for 2 units
            instrument_id
        );
        
        engine.process_order(buy_order, TimeInForce::GTC).unwrap();
        
        // Add a smaller sell order to create partial fill
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0), // Only 1 unit
            instrument_id
        );
        
        let result = engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Check trades
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].base_amount, dec!(1.0));
        
        // Check affected order (the buy order should be partially filled)
        assert_eq!(result.affected_orders.len(), 1);
        assert_eq!(result.affected_orders[0].status, OrderStatus::PartiallyFilled);
        assert_eq!(result.affected_orders[0].remaining_base, dec!(1.0));
        assert_eq!(result.affected_orders[0].filled_base, dec!(1.0));
        
        // Check if the order is still in the book
        let best_bid = engine.order_book.get_best_bid().unwrap();
        assert_eq!(best_bid.remaining_base, dec!(1.0));
    }

    #[test]
    fn test_multiple_partial_fills() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order for 5 units
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(5.0),
            instrument_id
        );
        
        let result = engine.process_order(buy_order, TimeInForce::GTC).unwrap();
        let buy_order_id = result.processed_order.unwrap().id;
        
        // Add 3 separate sell orders for 1 unit each
        for _ in 0..3 {
            let sell_order = create_test_order(
                Side::Ask, 
                OrderType::Limit, 
                Some(dec!(100.0)), 
                dec!(1.0),
                instrument_id
            );
            
            let result = engine.process_order(sell_order, TimeInForce::GTC).unwrap();
            assert_eq!(result.trades.len(), 1);
            assert_eq!(result.trades[0].base_amount, dec!(1.0));
            assert!(result.affected_orders.len() > 0);
        }
        
        // Verify the buy order is still partially filled
        let best_bid = engine.order_book.get_best_bid().unwrap();
        assert_eq!(best_bid.id, buy_order_id);
        assert_eq!(best_bid.remaining_base, dec!(2.0));
        assert_eq!(best_bid.filled_base, dec!(3.0));
        assert_eq!(best_bid.status, OrderStatus::PartiallyFilled);
    }
    
    #[test]
    fn test_price_improvement() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC sell order at 90
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(90.0)), 
            dec!(1.0),
            instrument_id
        );
        
        engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Add a buy order at 100 (should match at 90)
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        let result = engine.process_order(buy_order, TimeInForce::GTC).unwrap();
        
        // Verify buyer got price improvement
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].price, dec!(90.0)); // Matched at 90, not 100
    }
    
    #[test]
    fn test_wrong_instrument_rejection() {
        let instrument_id = Uuid::new_v4();
        let wrong_instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create order with wrong instrument ID
        let order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            wrong_instrument_id // Different from engine's instrument
        );
        
        let result = engine.process_order(order, TimeInForce::GTC);
        assert!(matches!(result, Err(MatchingError::WrongInstrument)));
    }
    
    #[test]
    fn test_invalid_limit_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create limit order with no price
        let mut order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        // Remove the price to make it invalid
        order.limit_price = None;
        
        let result = engine.process_order(order, TimeInForce::GTC);
        assert!(matches!(result, Err(MatchingError::InvalidOrder(_))));
    }
    
    #[test]
    fn test_cancel_nonexistent_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        let random_id = Uuid::new_v4();
        
        let result = engine.cancel_order(random_id);
        assert!(matches!(result, Err(MatchingError::OrderNotFound(_))));
    }
    
    #[test]
    fn test_cancel_partially_filled_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a GTC buy order for 2 units
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(2.0),
            instrument_id
        );
        
        let result = engine.process_order(buy_order, TimeInForce::GTC).unwrap();
        let buy_order_id = result.processed_order.unwrap().id;
        
        // Partially fill with 1 unit
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Cancel the partially filled order
        let cancelled = engine.cancel_order(buy_order_id).unwrap();
        assert_eq!(cancelled.status, OrderStatus::PartiallyFilledCancelled);
        assert_eq!(cancelled.filled_base, dec!(1.0));
        assert_eq!(cancelled.remaining_base, dec!(1.0));
    }
    
    #[test]
    fn test_price_time_priority() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add three buy orders at different prices
        let buy_order1 = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(101.0)), // Highest price
            dec!(1.0),
            instrument_id
        );
        
        let buy_order2 = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        let buy_order3 = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(99.0)), 
            dec!(1.0),
            instrument_id
        );
        
        // Add in reverse price order to ensure sorting works
        engine.process_order(buy_order3, TimeInForce::GTC).unwrap();
        engine.process_order(buy_order2, TimeInForce::GTC).unwrap();
        engine.process_order(buy_order1, TimeInForce::GTC).unwrap();
        
        // Add a sell order that should match the highest price first
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(99.0)), 
            dec!(1.0),
            instrument_id
        );
        
        let result = engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Verify matched with highest price (101)
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].price, dec!(101.0));
        
        // Best bid should now be the 100 price
        let best_bid = engine.order_book.get_best_bid().unwrap();
        assert_eq!(best_bid.limit_price.unwrap(), dec!(100.0));
    }
    
    #[test]
    fn test_same_price_time_priority() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add two buy orders at the same price
        let buy_order1 = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)),
            dec!(1.0),
            instrument_id
        );
        
        let result1 = engine.process_order(buy_order1.clone(), TimeInForce::GTC).unwrap();
        let first_order_id = result1.processed_order.unwrap().id;
        
        // Add second order with same price but later time
        let buy_order2 = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)),
            dec!(1.0),
            instrument_id
        );
        
        engine.process_order(buy_order2, TimeInForce::GTC).unwrap();
        
        // Add a sell order that should match the first buy order due to time priority
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        let result = engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Verify matched with first order
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].maker_order_id, first_order_id);
    }
    
    #[test]
    fn test_large_order_quantities() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a large sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1000000.0), // 1 million units
            instrument_id
        );
        
        engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Add a matching buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1000000.0),
            instrument_id
        );
        
        let result = engine.process_order(buy_order, TimeInForce::GTC).unwrap();
        
        // Verify matched correctly with large quantities
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].base_amount, dec!(1000000.0));
        assert_eq!(result.trades[0].quote_amount, dec!(100000000.0)); // 100M (1M * 100)
    }
    
    #[test]
    fn test_small_decimal_quantities() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Add a small quantity sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(0.0001), // Very small amount
            instrument_id
        );
        
        engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Add a matching buy order
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(0.0001),
            instrument_id
        );
        
        let result = engine.process_order(buy_order, TimeInForce::GTC).unwrap();
        
        // Verify matched correctly with small quantities
        assert_eq!(result.trades.len(), 1);
        assert_eq!(result.trades[0].base_amount, dec!(0.0001));
        assert_eq!(result.trades[0].quote_amount, dec!(0.0100)); // 0.0001 * 100
    }
    
    #[test]
    fn test_complex_matching_scenario() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create multiple orders at different price levels
        // Asks: 102, 103, 105
        // Bids: 98, 97, 95
        
        // Add ask orders
        let ask_orders = vec![
            create_test_order(Side::Ask, OrderType::Limit, Some(dec!(102.0)), dec!(1.0), instrument_id),
            create_test_order(Side::Ask, OrderType::Limit, Some(dec!(103.0)), dec!(2.0), instrument_id),
            create_test_order(Side::Ask, OrderType::Limit, Some(dec!(105.0)), dec!(3.0), instrument_id),
        ];
        
        // Add bid orders
        let bid_orders = vec![
            create_test_order(Side::Bid, OrderType::Limit, Some(dec!(98.0)), dec!(1.0), instrument_id),
            create_test_order(Side::Bid, OrderType::Limit, Some(dec!(97.0)), dec!(2.0), instrument_id),
            create_test_order(Side::Bid, OrderType::Limit, Some(dec!(95.0)), dec!(3.0), instrument_id),
        ];
        
        // Process orders
        for order in ask_orders {
            engine.process_order(order, TimeInForce::GTC).unwrap();
        }
        
        for order in bid_orders {
            engine.process_order(order, TimeInForce::GTC).unwrap();
        }
        
        // Verify order book state
        let best_ask = engine.order_book.get_best_ask().unwrap();
        let best_bid = engine.order_book.get_best_bid().unwrap();
        
        assert_eq!(best_ask.limit_price.unwrap(), dec!(102.0));
        assert_eq!(best_bid.limit_price.unwrap(), dec!(98.0));
        
        // Add an aggressive buy order that crosses multiple levels
        let aggressive_buy = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(104.0)), // This should match 102 and 103 but not 105
            dec!(5.0),
            instrument_id
        );
        
        let result = engine.process_order(aggressive_buy, TimeInForce::GTC).unwrap();
        
        // Should match against first two ask levels
        assert_eq!(result.trades.len(), 2);
        assert_eq!(result.trades[0].price, dec!(102.0)); // First match at 102
        assert_eq!(result.trades[1].price, dec!(103.0)); // Second match at 103
        assert_eq!(result.trades[0].base_amount, dec!(1.0));
        assert_eq!(result.trades[1].base_amount, dec!(2.0));
        
        // Verify remaining order quantity and status
        let processed = result.processed_order.unwrap();
        assert_eq!(processed.status, OrderStatus::PartiallyFilled);
        assert_eq!(processed.filled_base, dec!(3.0)); // 1 + 2
        assert_eq!(processed.remaining_base, dec!(2.0)); // 5 - 3
        
        // Best ask should now be 105
        let best_ask = engine.order_book.get_best_ask().unwrap();
        assert_eq!(best_ask.limit_price.unwrap(), dec!(105.0));
    }
    
    #[test]
    fn test_stop_order_processing() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create a stop order
        let mut stop_order = create_test_order(
            Side::Bid, 
            OrderType::Stop, 
            None, // Stop orders don't have limit price
            dec!(1.0),
            instrument_id
        );
        stop_order.trigger_price = Some(dec!(100.0));
        
        let result = engine.process_order(stop_order, TimeInForce::GTC).unwrap();
        
        // Check the stop order is correctly set to waiting trigger
        let processed = result.processed_order.unwrap();
        assert_eq!(processed.status, OrderStatus::WaitingTrigger);
        
        // No trades should have been generated
        assert_eq!(result.trades.len(), 0);
    }
    
    #[test]
    fn test_stop_limit_order_processing() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create a stop limit order
        let mut stop_limit_order = create_test_order(
            Side::Bid, 
            OrderType::StopLimit, 
            Some(dec!(99.0)), // Limit price
            dec!(1.0),
            instrument_id
        );
        stop_limit_order.trigger_price = Some(dec!(100.0)); // Trigger price
        
        let result = engine.process_order(stop_limit_order, TimeInForce::GTC).unwrap();
        
        // Check the stop limit order is correctly set to waiting trigger
        let processed = result.processed_order.unwrap();
        assert_eq!(processed.status, OrderStatus::WaitingTrigger);
        
        // No trades should have been generated
        assert_eq!(result.trades.len(), 0);
    }
    
    #[test]
    fn test_invalid_stop_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Create a stop order without trigger price (invalid)
        let stop_order = create_test_order(
            Side::Bid, 
            OrderType::Stop, 
            None,
            dec!(1.0),
            instrument_id
        );
        // Intentionally not setting trigger_price
        
        let result = engine.process_order(stop_order, TimeInForce::GTC);
        assert!(matches!(result, Err(MatchingError::InvalidOrder(_))));
    }
    
    #[test]
    fn test_invalid_stop_limit_order() {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Case 1: Missing trigger price
        let stop_limit_order1 = create_test_order(
            Side::Bid, 
            OrderType::StopLimit, 
            Some(dec!(99.0)), // Has limit price
            dec!(1.0),
            instrument_id
        );
        // Intentionally not setting trigger_price
        
        let result1 = engine.process_order(stop_limit_order1, TimeInForce::GTC);
        assert!(matches!(result1, Err(MatchingError::InvalidOrder(_))));
        
        // Case 2: Missing limit price
        let mut stop_limit_order2 = create_test_order(
            Side::Bid, 
            OrderType::StopLimit, 
            None, // No limit price
            dec!(1.0),
            instrument_id
        );
        stop_limit_order2.trigger_price = Some(dec!(100.0));
        
        let result2 = engine.process_order(stop_limit_order2, TimeInForce::GTC);
        assert!(matches!(result2, Err(MatchingError::InvalidOrder(_))));
    }
    
    // === EVENT SYSTEM INTEGRATION TESTS ===
    
    #[tokio::test]
    async fn test_event_integration() {
        use tokio::sync::Mutex;
        use std::sync::Arc;
        use crate::events::{EventBus, MatchingEngineEvent, EventHandler, EventResult};
        
        // Create a simple event collector
        struct EventCollector {
            events: Mutex<Vec<MatchingEngineEvent>>,
        }
        
        #[async_trait::async_trait]
        impl EventHandler for EventCollector {
            fn event_types(&self) -> Vec<&'static str> {
                vec![
                    "OrderAdded", 
                    "OrderMatched", 
                    "OrderCancelled", 
                    "TradeExecuted",
                    "DepthUpdated"
                ]
            }
            
            async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()> {
                let mut events = self.events.lock().await;
                events.push(event);
                Ok(())
            }
        }
        
        // Setup test
        let instrument_id = Uuid::new_v4();
        let event_bus = EventBus::default();
        let collector = Arc::new(EventCollector {
            events: Mutex::new(Vec::new()),
        });
        
        // Create the dispatcher and register the collector
        let dispatcher = crate::events::EventDispatcher::new(event_bus.clone());
        dispatcher.register_handler(collector.clone()).await;
        let _handle = dispatcher.start().await;
        
        // Create the matching engine with the event bus
        let mut engine = MatchingEngine::with_event_bus(instrument_id, event_bus);
        
        // Process an order
        let order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        engine.process_order(order, TimeInForce::GTC).unwrap();
        
        // Get depth to trigger DepthUpdated event
        engine.get_depth(10);
        
        // Allow time for events to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Check collected events
        let events = collector.events.lock().await;
        
        // Should have at least 2 events: OrderAdded and DepthUpdated
        assert!(events.len() >= 2);
        
        // Verify we have the right event types
        let mut has_order_added = false;
        let mut has_depth_updated = false;
        
        for event in events.iter() {
            match event {
                MatchingEngineEvent::OrderAdded { .. } => has_order_added = true,
                MatchingEngineEvent::DepthUpdated { .. } => has_depth_updated = true,
                _ => {}
            }
        }
        
        assert!(has_order_added, "Missing OrderAdded event");
        assert!(has_depth_updated, "Missing DepthUpdated event");
    }
    
    #[tokio::test]
    async fn test_event_trade_execution() {
        use tokio::sync::Mutex;
        use std::sync::Arc;
        use crate::events::{EventBus, MatchingEngineEvent, EventHandler, EventResult};
        
        // Create a simple event collector
        struct EventCollector {
            events: Mutex<Vec<MatchingEngineEvent>>,
        }
        
        #[async_trait::async_trait]
        impl EventHandler for EventCollector {
            fn event_types(&self) -> Vec<&'static str> {
                vec![
                    "OrderAdded", 
                    "OrderMatched", 
                    "OrderCancelled", 
                    "TradeExecuted",
                    "DepthUpdated"
                ]
            }
            
            async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()> {
                let mut events = self.events.lock().await;
                events.push(event);
                Ok(())
            }
        }
        
        // Setup test
        let instrument_id = Uuid::new_v4();
        let event_bus = EventBus::default();
        let collector = Arc::new(EventCollector {
            events: Mutex::new(Vec::new()),
        });
        
        // Create the dispatcher and register the collector
        let dispatcher = crate::events::EventDispatcher::new(event_bus.clone());
        dispatcher.register_handler(collector.clone()).await;
        let _handle = dispatcher.start().await;
        
        // Create the matching engine with the event bus
        let mut engine = MatchingEngine::with_event_bus(instrument_id, event_bus);
        
        // Add a GTC sell order
        let sell_order = create_test_order(
            Side::Ask, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        engine.process_order(sell_order, TimeInForce::GTC).unwrap();
        
        // Add a matching buy order to generate a trade
        let buy_order = create_test_order(
            Side::Bid, 
            OrderType::Limit, 
            Some(dec!(100.0)), 
            dec!(1.0),
            instrument_id
        );
        
        engine.process_order(buy_order, TimeInForce::GTC).unwrap();
        
        // Allow time for events to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Check collected events
        let events = collector.events.lock().await;
        
        // Verify we have a trade execution event
        let mut has_trade_executed = false;
        let mut has_order_matched = false;
        
        for event in events.iter() {
            match event {
                MatchingEngineEvent::TradeExecuted { .. } => has_trade_executed = true,
                MatchingEngineEvent::OrderMatched { .. } => has_order_matched = true,
                _ => {}
            }
        }
        
        assert!(has_trade_executed, "Missing TradeExecuted event");
        assert!(has_order_matched, "Missing OrderMatched event");
    }
}

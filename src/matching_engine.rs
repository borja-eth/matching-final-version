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
}

impl MatchingEngine {
    /// Creates a new matching engine for a specific instrument.
    pub fn new(instrument_id: Uuid) -> Self {
        Self {
            order_book: OrderBook::new(instrument_id),
            order_index: HashMap::new(),
            next_sequence_id: 1,
            instrument_id,
        }
    }
    
    /// Processes a new order through the matching engine.
    ///
    /// # Arguments
    /// * `order` - The order to process
    /// * `time_in_force` - Duration policy for the order
    ///
    /// # Returns
    /// A `MatchingResult` containing the trades generated and the state of the order after processing
    pub fn process_order(&mut self, mut order: Order, time_in_force: TimeInForce) -> MatchingResult<MatchResult> {
        // Validate the order
        if order.instrument_id != self.instrument_id {
            return Err(MatchingError::InvalidOrder(
                format!("Order instrument ID does not match engine (expected: {}, got: {})",
                        self.instrument_id, order.instrument_id)
            ));
        }
        
        // Assign sequence ID for time priority
        order.sequence_id = self.next_sequence_id;
        self.next_sequence_id += 1;
        
        // Handle market orders as aggressive IOC orders
        let effective_tif = if order.order_type == OrderType::Market {
            TimeInForce::IOC
        } else {
            time_in_force
        };
        
        // For market orders, ensure they don't have a limit price
        if order.order_type == OrderType::Market {
            order.limit_price = None;
        }
        
        // For limit orders, ensure they have a limit price
        if order.order_type == OrderType::Limit && order.limit_price.is_none() {
            return Err(MatchingError::InvalidOrder("Limit order must have a price".into()));
        }
        
        // Match the order against the book
        let mut result = self.match_order(&mut order)?;
        
        // If it's an IOC order and not fully filled, cancel the remainder
        if effective_tif == TimeInForce::IOC && order.status != OrderStatus::Filled {
            // For IOC, we don't add to the book, just mark it cancelled
            if order.status == OrderStatus::New {
                order.status = OrderStatus::Cancelled;
            } else {
                order.status = OrderStatus::PartiallyFilledCancelled;
            }
        } 
        // If GTC and not fully filled, add to the book
        else if order.status != OrderStatus::Filled {
            // Add remaining order to the book
            self.add_to_book(&order);
        }
        
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
    fn match_order(&mut self, order: &mut Order) -> MatchingResult<MatchResult> {
        let mut result = MatchResult::default();
        
        // Find the opposite side
        let opposite_side = match order.side {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        };
        
        // Keep matching until the order is filled or no more matches are possible
        loop {
            // Exit if order is fully filled
            if order.remaining_base.is_zero() {
                order.status = OrderStatus::Filled;
                break;
            }
            
            // Get the best opposing order
            let best_opposing_order = match opposite_side {
                Side::Bid => self.order_book.get_best_bid(),
                Side::Ask => self.order_book.get_best_ask(),
            };
            
            // Check if there's no matching order
            if best_opposing_order.is_none() {
                break;
            }
            
            // For limit orders, check if the price is acceptable
            if order.order_type == OrderType::Limit {
                let limit_price = match order.limit_price {
                    Some(price) => price,
                    None => return Err(MatchingError::InvalidOrder("Limit order must have a price".to_string())),
                };
                
                let best_opposing_order_ref = match best_opposing_order {
                    Some(order) => order,
                    None => break,
                };
                
                let best_price = match best_opposing_order_ref.limit_price {
                    Some(price) => price,
                    None => return Err(MatchingError::InvalidOrder("Opposing order must have a price".to_string())),
                };
                
                // Check if price is acceptable based on order side
                let price_acceptable = match order.side {
                    Side::Bid => best_price <= limit_price, // Buy: best ask <= my bid
                    Side::Ask => best_price >= limit_price, // Sell: best bid >= my ask
                };
                
                if !price_acceptable {
                    break;
                }
            }
            
            // Get the best opposing order and remove it from the book
            let best_opposing_order_ref = match best_opposing_order {
                Some(order) => order,
                None => break,
            };
            
            let best_price = match best_opposing_order_ref.limit_price {
                Some(price) => price,
                None => return Err(MatchingError::InvalidOrder("Opposing order must have a price".to_string())),
            };
            
            let opposing_order_id = best_opposing_order_ref.id;
            
            let mut opposing_order = match self.order_book.remove_order(
                opposing_order_id,
                opposite_side,
                best_price
            ) {
                Some(order) => order,
                None => return Err(MatchingError::OrderNotFound(opposing_order_id)),
            };
            
            // Remove from our index
            self.order_index.remove(&opposing_order.id);
            
            // Calculate matched quantity
            let matched_qty = Decimal::min(order.remaining_base, opposing_order.remaining_base);
            
            // Calculate quote amount
            let quote_amount = matched_qty * best_price;
            
            // Create trade record
            let trade = Trade {
                id: Uuid::new_v4(),
                instrument_id: self.instrument_id,
                maker_order_id: opposing_order.id,
                taker_order_id: order.id,
                base_amount: matched_qty,
                quote_amount,
                price: best_price,
                created_at: Utc::now(),
            };
            
            // Update order states
            order.remaining_base -= matched_qty;
            order.filled_base += matched_qty;
            order.filled_quote += quote_amount;
            
            opposing_order.remaining_base -= matched_qty;
            opposing_order.filled_base += matched_qty;
            opposing_order.filled_quote += quote_amount;
            
            // Update order statuses
            if order.status == OrderStatus::New && !order.remaining_base.is_zero() {
                order.status = OrderStatus::PartiallyFilled;
            }
            
            if opposing_order.remaining_base.is_zero() {
                opposing_order.status = OrderStatus::Filled;
            } else {
                opposing_order.status = OrderStatus::PartiallyFilled;
                // Put partially filled order back in the book
                self.add_to_book(&opposing_order);
            }
            
            // Record trade and affected order
            result.trades.push(trade);
            result.affected_orders.push(opposing_order);
        }
        
        // For market orders with no matches, return an error
        if order.order_type == OrderType::Market && 
           order.status == OrderStatus::New && 
           result.trades.is_empty() {
            return Err(MatchingError::InsufficientLiquidity);
        }
        
        Ok(result)
    }
    
    /// Adds an order to the book and updates the index.
    fn add_to_book(&mut self, order: &Order) {
        if let Some(price) = order.limit_price {
            self.order_book.add_order(order.clone());
            self.order_index.insert(order.id, (order.side, price));
        }
    }
    
    /// Cancels an existing order in the order book.
    ///
    /// # Arguments
    /// * `order_id` - The ID of the order to cancel
    ///
    /// # Returns
    /// The cancelled order if found
    pub fn cancel_order(&mut self, order_id: Uuid) -> MatchingResult<Order> {
        // Look up the order location in our index
        if let Some((side, price)) = self.order_index.remove(&order_id) {
            if let Some(mut order) = self.order_book.remove_order(order_id, side, price) {
                // Update order status
                if order.status == OrderStatus::PartiallyFilled {
                    order.status = OrderStatus::PartiallyFilledCancelled;
                } else {
                    order.status = OrderStatus::Cancelled;
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
}

//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements a limit order book for a single trading instrument.
// It maintains bid and ask orders in price-time priority (FIFO) order.
//
// | Component     | Description                                                               |
// |--------------|---------------------------------------------------------------------------|
// | OrderBook    | Main order book structure managing bids and asks                          |
// | PriceLevel   | Groups orders at the same price level                                     |
// | FIFO Queue   | Orders within each price level are processed first-in-first-out          |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name          | Description                                        | Key Methods              |
// |---------------|----------------------------------------------------|-------------------------|
// | PriceLevel    | Maintains orders at a specific price              | peek_next_order         |
// |               |                                                    | is_empty                |
// |               |                                                    | order_count             |
// |--------------|---------------------------------------------------|-------------------------|
// | OrderBook     | Main order book implementation                    | add_order               |
// |               |                                                    | remove_order            |
// |               |                                                    | peek_best_order         |
// |               |                                                    | get_orders_at_price     |
//
//--------------------------------------------------------------------------------------------------
// FUNCTIONS
//--------------------------------------------------------------------------------------------------
// | Name                  | Description                               | Return Type             |
// |-----------------------|-------------------------------------------|------------------------|
// | new                   | Creates new OrderBook                     | OrderBook             |
// | add_order            | Adds order to book                        | Result<(), OrderBookError> |
// | remove_order         | Removes order from book                   | Option<Order>         |
// | peek_best_order      | Gets next order without removing         | Option<&Order>        |
// | best_bid             | Gets best bid price                      | Option<Decimal>       |
// | best_ask             | Gets best ask price                      | Option<Decimal>       |
// | spread               | Gets current spread                      | Option<Decimal>       |
// | volume_at_price      | Gets volume at price level              | Option<Decimal>       |
//
//--------------------------------------------------------------------------------------------------
// TESTS
//--------------------------------------------------------------------------------------------------
// | Name                          | Description                                              |
// |-------------------------------|----------------------------------------------------------|
// | test_empty_orderbook         | Verifies initial empty state                            |
// | test_single_order            | Tests single order operations                           |
// | test_multiple_orders         | Tests multiple orders at same price                     |
// | test_price_levels            | Tests orders at different prices                        |
// | test_remove_order            | Tests order removal                                     |
// | test_spread_calculation      | Tests spread calculations                               |
// | test_fifo_order_execution    | Tests FIFO ordering of orders                          |
// | test_order_count_tracking    | Tests order counting at price levels                    |
//--------------------------------------------------------------------------------------------------

use std::collections::{BTreeMap, VecDeque, HashMap};
use uuid::Uuid;

// Import types from types.rs
use crate::domain::models::types::{Order, Side};

/// Represents a price level in the order book, maintaining a FIFO queue of orders
/// at the same price point.
#[derive(Debug, Clone)]
pub struct PriceLevel {
    /// The price for this level
    pub price: i64,
    /// FIFO queue of orders at this price level
    pub orders: VecDeque<Order>,
    /// Total volume of all orders at this price level
    pub total_volume: u64,
}

impl PriceLevel {
    /// Creates a new price level with the given price.
    ///
    /// # Arguments
    /// * `price` - The price for this level
    /// * `initial_capacity` - Optional capacity for the order queue
    ///
    /// # Returns
    /// A new price level with the given price and an empty order queue
    pub fn new(price: i64, initial_capacity: Option<usize>) -> Self {
        let capacity = initial_capacity.unwrap_or(4);
        Self {
            price,
            orders: VecDeque::with_capacity(capacity),
            total_volume: 0,
        }
    }

    /// Returns the next order to be matched without removing it from the queue.
    /// This maintains FIFO ordering by always returning the front of the queue.
    ///
    /// # Returns
    /// * `Some(&Order)` - Reference to the next order to be matched
    /// * `None` - If there are no orders at this price level
    #[inline]
    pub fn peek_next_order(&self) -> Option<&Order> {
        self.orders.front()
    }

    /// Returns true if this price level has no orders.
    ///
    /// # Returns
    /// * `true` - If there are no orders at this price level
    /// * `false` - If there are orders at this price level
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    /// Returns the number of orders at this price level.
    ///
    /// # Returns
    /// * `usize` - The count of orders at this price level
    pub fn order_count(&self) -> usize {
        self.orders.len()
    }
}

/// The main order book structure that maintains bid and ask orders in price-time priority.
/// Uses BTreeMap for price level organization and VecDeque for FIFO ordering within price levels.
#[derive(Debug)]
pub struct OrderBook {
    /// Bid side orders organized by price (descending)
    bids: BTreeMap<i64, PriceLevel>,
    /// Ask side orders organized by price (ascending)
    asks: BTreeMap<i64, PriceLevel>,
    /// Cache of best bid price for quick access
    /// This is an Option because the order book may be empty or have no bids,
    /// in which case there is no best bid price to reference.
    best_bid: Option<i64>,
    /// Cache of best ask price for quick access
    best_ask: Option<i64>,
    /// Identifier for the instrument this order book manages
    instrument_id: Uuid,
    /// O(1) lookup for orders by ID
    order_map: HashMap<Uuid, (Side, i64)>,
}

impl OrderBook {
    /// Creates a new empty order book for a specific instrument.
    ///
    /// # Arguments
    /// * `instrument_id` - The unique identifier of the instrument this order book will manage
    ///
    /// # Returns
    /// A new `OrderBook` instance with empty bid and ask sides
    pub fn new(instrument_id: Uuid) -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            best_bid: None,
            best_ask: None,
            instrument_id,
            order_map: HashMap::new(),
        }
    }

    /// Gets an order by its ID with O(1) complexity.
    ///
    /// # Arguments
    /// * `order_id` - The unique identifier of the order to find
    ///
    /// # Returns
    /// * `Some(&Order)` - Reference to the found order
    /// * `None` - If no order exists with the given ID
    pub fn get_order_by_id(&self, order_id: &Uuid) -> Option<&Order> {
        self.order_map.get(order_id).and_then(|(side, price)| {
            let price_levels = match side {
                Side::Bid => &self.bids,
                Side::Ask => &self.asks,
            };
            price_levels.get(price).and_then(|level| {
                level.orders.iter().find(|order| order.id == *order_id)
            })
        })
    }

    /// Adds a new order to the order book in price-time priority.
    /// Orders are organized first by price (best price first) and then by time of arrival (FIFO).
    ///
    /// # Arguments
    /// * `order` - The order to add to the book
    ///
    /// # Returns
    /// * `Ok(())` - If the order was successfully added
    /// * `Err(OrderBookError)` - If the order could not be added
    ///
    /// # Notes
    /// - Orders for different instruments are rejected
    /// - Market orders (no limit price) are rejected
    /// - Orders are added to the back of the queue at their price level
    /// - Best prices are automatically updated
    #[inline(always)]
    pub fn add_order(&mut self, order: Order) -> Result<(), OrderBookError> {
        // 1. Fast-path validation (most common checks first)
        let price = match order.limit_price {
            Some(p) => p,
            None => return Err(OrderBookError::NoLimitPrice),
        };

        if order.instrument_id != self.instrument_id {
            return Err(OrderBookError::WrongInstrument {
                expected: self.instrument_id,
                got: order.instrument_id,
            });
        }

        // 2. Direct access to correct price level map (no match overhead)
        let price_levels = if order.side == Side::Bid {
            &mut self.bids
        } else {
            &mut self.asks
        };

        // 3. Reserve capacity in the price level's orders to avoid reallocation
        let price_level = price_levels.entry(price).or_insert_with(|| {
            // Use the constructor to create a new price level
            PriceLevel::new(price, Some(4))
        });

        // 4. Update price level (no cloning of the entire order)
        price_level.total_volume = price_level.total_volume.saturating_add(order.remaining_base);
        price_level.orders.push_back(order.clone());

        // 5. O(1) lookup map update (moved before best price update as it's faster)
        self.order_map.insert(order.id, (order.side, price));

        // 6. Update best prices cache only if needed
        match order.side {
            Side::Bid if self.best_bid.map_or(true, |p| price > p) => self.best_bid = Some(price),
            Side::Ask if self.best_ask.map_or(true, |p| price < p) => self.best_ask = Some(price),
            _ => {}
        }

        Ok(())
    }

    /// Removes an order from the order book.
    ///
    /// # Arguments
    /// * `order_id` - The unique identifier of the order to remove
    ///
    /// # Returns
    /// * `Ok(Order)` - The removed order
    /// * `Err(OrderBookError)` - If the order was not found or other error occurred
    ///
    /// # Performance
    /// This operation is O(1) for order lookup and O(1) for removal from price level
    /// as we maintain a direct lookup map to the order's location.
    #[inline]
    pub fn remove_order(&mut self, order_id: Uuid) -> Result<Order, OrderBookError> {
        // 1. Fast lookup of order details using O(1) map
        let (side, price) = self.order_map.remove(&order_id)
            .ok_or(OrderBookError::OrderNotFound(order_id))?;

        // 2. Direct access to correct price level map
        let price_levels = if side == Side::Bid {
            &mut self.bids
        } else {
            &mut self.asks
        };

        // 3. Get price level
        let price_level = price_levels.get_mut(&price)
            .ok_or(OrderBookError::InvalidPrice(price))?;

        // 4. Find and remove order in one pass
        let order_idx = price_level.orders.iter()
            .position(|o| o.id == order_id)
            .ok_or(OrderBookError::OrderNotFound(order_id))?;

        let order = price_level.orders.remove(order_idx)
            .ok_or(OrderBookError::OrderNotFound(order_id))?;

        // 5. Update volume (using saturating_sub for safety)
        price_level.total_volume = price_level.total_volume.saturating_sub(order.remaining_base);

        // 6. Clean up empty price level if needed
        if price_level.orders.is_empty() {
            price_levels.remove(&price);
            
            // 7. Update best prices only if needed
            match side {
                Side::Bid if Some(price) == self.best_bid => self.update_best_bid(),
                Side::Ask if Some(price) == self.best_ask => self.update_best_ask(),
                _ => {}
            }
        }

        Ok(order)
    }

    /// Updates only the best bid price
    #[inline(always)]
    fn update_best_bid(&mut self) {
        self.best_bid = self.bids.keys().next_back().copied();
    }

    /// Updates only the best ask price
    #[inline(always)]
    fn update_best_ask(&mut self) {
        self.best_ask = self.asks.keys().next().copied();
    }

    /// Gets the next order to be matched without removing it from the book.
    ///
    /// # Arguments
    /// * `side` - The side of the order book to peek (Bid or Ask)
    ///
    /// # Returns
    /// * `Some(&Order)` - Reference to the next order to be matched
    /// * `None` - If there are no orders on the specified side
    ///
    /// # Notes
    /// - For bids, returns the highest priced order
    /// - For asks, returns the lowest priced order
    /// - Within a price level, returns the first order (FIFO)
    #[inline]
    pub fn peek_best_order(&self, side: Side) -> Option<&Order> {
        let (price_levels, best_price) = match side {
            Side::Bid => (&self.bids, self.best_bid),
            Side::Ask => (&self.asks, self.best_ask),
        };

        best_price.and_then(|price| {
            price_levels.get(&price).and_then(|level| level.peek_next_order())
        })
    }

    /// Returns all orders at a specific price level in FIFO order.
    ///
    /// # Arguments
    /// * `side` - The side (Bid/Ask) to look up
    /// * `price` - The price level to retrieve orders from
    ///
    /// # Returns
    /// * `Some(&VecDeque<Order>)` - Reference to the queue of orders at the specified price
    /// * `None` - If no orders exist at the specified price
    pub fn get_orders_at_price(&self, side: Side, price: i64) -> Option<&VecDeque<Order>> {
        let price_levels = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };
        price_levels.get(&price).map(|level| &level.orders)
    }

    /// Returns the number of orders at a specific price level.
    ///
    /// # Arguments
    /// * `side` - The side (Bid/Ask) to look up
    /// * `price` - The price level to count orders at
    ///
    /// # Returns
    /// * `usize` - The number of orders at the specified price level
    pub fn order_count_at_price(&self, side: Side, price: i64) -> usize {
        let price_levels = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };
        price_levels.get(&price).map_or(0, |level| level.order_count())
    }

    /// Returns the best bid price.
    ///
    /// # Returns
    /// * `Some(i64)` - The highest bid price with orders
    /// * `None` - If there are no bid orders
    #[inline]
    pub fn best_bid(&self) -> Option<i64> {
        self.best_bid
    }

    /// Returns the best ask price.
    ///
    /// # Returns
    /// * `Some(i64)` - The lowest ask price with orders
    /// * `None` - If there are no ask orders
    #[inline]
    pub fn best_ask(&self) -> Option<i64> {
        self.best_ask
    }

    /// Returns the spread between the best bid and ask prices.
    ///
    /// # Returns
    /// * `Some(i64)` - The difference between best ask and best bid
    /// * `None` - If either best bid or best ask is missing
    pub fn spread(&self) -> Option<i64> {
        match (self.best_ask, self.best_bid) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Returns the total volume at a specific price level.
    ///
    /// # Arguments
    /// * `side` - The side (Bid/Ask) to look up
    /// * `price` - The price level to get volume for
    ///
    /// # Returns
    /// * `Some(u64)` - The total volume at the specified price
    /// * `None` - If no orders exist at the specified price
    pub fn volume_at_price(&self, side: Side, price: i64) -> Option<u64> {
        let price_levels = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };
        price_levels.get(&price).map(|level| level.total_volume)
    }

    /// Returns the instrument ID this order book manages.
    ///
    /// # Returns
    /// * `Uuid` - The unique identifier of the instrument
    pub fn instrument_id(&self) -> Uuid {
        self.instrument_id
    }

    /// Gets the best bid order without removing it from the book.
    ///
    /// # Returns
    /// * `Some(&Order)` - Reference to the best bid order (highest price)
    /// * `None` - If there are no bid orders
    #[inline]
    pub fn get_best_bid(&self) -> Option<&Order> {
        self.peek_best_order(Side::Bid)
    }

    /// Gets the best ask order without removing it from the book.
    ///
    /// # Returns
    /// * `Some(&Order)` - Reference to the best ask order (lowest price)
    /// * `None` - If there are no ask orders
    #[inline]
    pub fn get_best_ask(&self) -> Option<&Order> {
        self.peek_best_order(Side::Ask)
    }

    /// Gets a reference to all orders at a certain price level.
    ///
    /// # Returns
    /// A reference to the price level at the specified price, or None if no orders exist.
    pub fn get_price_level(&self, side: Side, price: i64) -> Option<&PriceLevel> {
        match side {
            Side::Bid => self.bids.get(&price),
            Side::Ask => self.asks.get(&price),
        }
    }
    
    /// Gets a batch of best opposing orders for efficient matching.
    ///
    /// # Arguments
    /// * `side` - The side of orders to get (opposite of the incoming order)
    /// * `limit` - Maximum number of price levels to return
    ///
    /// # Returns
    /// A vector of price levels and their orders, sorted by price-time priority
    pub fn get_best_opposing_levels(&self, side: Side, limit: usize) -> Vec<(i64, &PriceLevel)> {
        let mut result = Vec::with_capacity(limit);
        
        match side {
            Side::Bid => {
                // For bids (buy orders), get highest prices first (descending)
                for (price, level) in self.bids.iter().rev().take(limit) {
                    if !level.orders.is_empty() {
                        result.push((*price, level));
                    }
                    
                    if result.len() >= limit {
                        break;
                    }
                }
            },
            Side::Ask => {
                // For asks (sell orders), get lowest prices first (ascending)
                for (price, level) in self.asks.iter().take(limit) {
                    if !level.orders.is_empty() {
                        result.push((*price, level));
                    }
                    
                    if result.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        result
    }
}

/// Errors that can occur during order book operations
#[derive(Debug, thiserror::Error)]
pub enum OrderBookError {
    /// Order is for a different instrument than this order book
    #[error("Order is for wrong instrument (expected {expected}, got {got})")]
    WrongInstrument {
        expected: Uuid,
        got: Uuid,
    },

    /// Market orders cannot be added to the book
    #[error("Market orders cannot be added to the order book (no limit price)")]
    NoLimitPrice,

    /// Order not found in the book
    #[error("Order {0} not found in the book")]
    OrderNotFound(Uuid),

    /// Invalid price level
    #[error("Invalid price level: {0}")]
    InvalidPrice(i64),

    /// Invalid order quantity
    #[error("Invalid order quantity: {0}")]
    InvalidQuantity(u64),
}

#[cfg(test)]
mod tests {
    //--------------------------------------------------------------------------------------------------
    // TEST MODULE OVERVIEW
    //--------------------------------------------------------------------------------------------------
    // This module contains comprehensive tests for the OrderBook implementation.
    // Tests are organized into categories:
    //
    // 1. Basic Functionality
    //    - Empty orderbook state
    //    - Single order operations
    //    - Multiple orders
    //
    // 2. Price Level Management
    //    - Multiple price levels
    //    - Volume tracking
    //    - Best price updates
    //
    // 3. FIFO Ordering
    //    - Order priority
    //    - Sequence tracking
    //    - Order removal effects
    //
    // 4. Edge Cases
    //    - Zero quantity
    //    - Large numbers
    //    - Invalid operations
    //--------------------------------------------------------------------------------------------------

    use super::*;
    use crate::domain::models::types::{OrderType, CreatedFrom};
    use chrono::Utc;
    use crate::domain::models::types::OrderStatus;
    use crate::domain::models::types::TimeInForce;

    /// Creates a test order with the specified parameters.
    fn create_test_order(side: Side, price: i64, quantity: u64, instrument_id: Uuid) -> Order {
        let now = Utc::now();
        Order {
            id: Uuid::new_v4(),
            ext_id: Some("test-order".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Limit,
            instrument_id,
            side,
            limit_price: Some(price),
            trigger_price: None,
            base_amount: quantity,
            remaining_base: quantity,
            filled_quote: 0,
            filled_base: 0,
            remaining_quote: price as u64 * quantity,
            expiration_date: now + chrono::Duration::days(365),
            status: OrderStatus::Submitted,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 1,
            time_in_force: TimeInForce::GTC,
        }
    }

    /// Tests that a new orderbook is properly initialized empty.
    #[test]
    fn test_empty_orderbook() {
        let instrument_id = Uuid::new_v4();
        let book = OrderBook::new(instrument_id);
        
        assert_eq!(book.best_bid(), None);
        assert_eq!(book.best_ask(), None);
        assert_eq!(book.spread(), None);
        assert_eq!(book.volume_at_price(Side::Bid, 100), None);
        assert_eq!(book.volume_at_price(Side::Ask, 100), None);
    }

    /// Tests basic operations with a single order.
    #[test]
    fn test_single_order() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        let order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        book.add_order(order.clone()).unwrap();
        
        assert_eq!(book.best_bid(), Some(100_000));
        assert_eq!(book.best_ask(), None);
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), Some(100_000));
    }

    /// Tests handling of multiple orders at the same price level.
    #[test]
    fn test_multiple_orders_same_price() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add multiple orders at same price
        for _ in 0..5 {
            let order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
            book.add_order(order).unwrap();
        }
        
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), Some(500_000));
    }

    /// Tests order management across different price levels.
    #[test]
    fn test_price_levels() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add orders at different price levels
        let prices = [100_000, 99_000, 101_000];
        for price in prices {
            let order = create_test_order(Side::Bid, price, 100_000, instrument_id);
            book.add_order(order).unwrap();
        }
        
        assert_eq!(book.best_bid(), Some(101_000)); // Highest bid
    }

    /// Tests order removal functionality.
    #[test]
    fn test_remove_order() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        let order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        book.add_order(order.clone()).unwrap();
        
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), Some(100_000));
        
        let removed = book.remove_order(order.id);
        assert!(removed.is_ok());
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), None);
    }

    /// Tests handling of non-existent order removal.
    #[test]
    fn test_remove_nonexistent_order() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        let order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        book.add_order(order.clone()).unwrap();
        
        // Try to remove with wrong price
        let removed = book.remove_order(order.id);
        assert!(removed.is_err());
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), Some(100_000));
    }

    /// Tests spread calculation between bid and ask sides.
    #[test]
    fn test_spread_calculation() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add bid and ask orders
        let bid_order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        let ask_order = create_test_order(Side::Ask, 101_000, 100_000, instrument_id);
        
        book.add_order(bid_order).unwrap();
        book.add_order(ask_order).unwrap();
        
        assert_eq!(book.spread(), Some(1_000));
    }

    /// Tests handling of orders for wrong instrument IDs.
    #[test]
    fn test_wrong_instrument_id() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Try to add order for different instrument
        let wrong_instrument_id = Uuid::new_v4();
        let order = create_test_order(Side::Bid, 100_000, 100_000, wrong_instrument_id);
        
        let _ = book.add_order(order);
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), None);
    }

    /// Tests volume tracking across multiple price levels.
    #[test]
    fn test_multiple_price_levels_volume() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add orders at different price levels
        let price_levels = [
            (100_000, 200_000),  // price, quantity
            (99_000, 300_000),
            (101_000, 100_000),
        ];
        
        for (price, quantity) in price_levels {
            let order = create_test_order(Side::Bid, price, quantity, instrument_id);
            book.add_order(order).unwrap();
        }
        
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), Some(200_000));
        assert_eq!(book.volume_at_price(Side::Bid, 99_000), Some(300_000));
        assert_eq!(book.volume_at_price(Side::Bid, 101_000), Some(100_000));
    }

    /// Tests FIFO ordering of orders within price levels.
    #[test]
    fn test_fifo_order_execution() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add orders with different sequence IDs but same price
        let mut orders = Vec::new();
        for i in 1..=3 {
            let mut order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
            order.sequence_id = i;
            orders.push(order.clone());
            book.add_order(order).unwrap();
        }

        // Verify get_best_bid returns the first order
        let best_order = book.get_best_bid().expect("Expected to find a best bid order");
        assert_eq!(best_order.sequence_id, 1);

        // Remove first order and verify next one becomes best
        book.remove_order(orders[0].id).unwrap();
        let next_best = book.get_best_bid().expect("Expected to find a next best bid order");
        assert_eq!(next_best.sequence_id, 2);
    }

    /// Tests order counting functionality at price levels.
    #[test]
    fn test_order_count_tracking() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add multiple orders at same price
        for _ in 0..3 {
            let order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
            book.add_order(order).unwrap();
        }

        assert_eq!(book.order_count_at_price(Side::Bid, 100_000), 3);
        
        // Add orders at different price
        let order = create_test_order(Side::Bid, 101_000, 100_000, instrument_id);
        book.add_order(order).unwrap();
        
        assert_eq!(book.order_count_at_price(Side::Bid, 101_000), 1);
    }

    /// Tests various edge cases in order handling.
    #[test]
    fn test_edge_cases() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Test zero quantity
        let zero_order = create_test_order(Side::Bid, 100_000, 0, instrument_id);
        book.add_order(zero_order).unwrap();
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), Some(0));
        
        // Test very large quantity
        let large_order = create_test_order(Side::Bid, 100_000, 1_000_000_000, instrument_id);
        book.add_order(large_order).unwrap();
        assert_eq!(book.volume_at_price(Side::Bid, 100_000), Some(1_000_000_000));
        
        // Test minimum price
        let min_price_order = create_test_order(Side::Bid, 1, 100_000, instrument_id);
        book.add_order(min_price_order).unwrap();
        assert_eq!(book.volume_at_price(Side::Bid, 1), Some(100_000));
    }
}
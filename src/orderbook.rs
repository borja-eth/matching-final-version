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
// | add_order            | Adds order to book                        | ()                    |
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

use std::collections::{BTreeMap, VecDeque};
use rust_decimal::Decimal;
use uuid::Uuid;

// Import types from types.rs
use crate::types::{Order, Side, OrderStatus};

/// Represents a price level in the order book, maintaining a FIFO queue of orders
/// at the same price point.
#[derive(Debug, Clone)]
pub struct PriceLevel {
    /// The price for this level
    pub price: Decimal,
    /// FIFO queue of orders at this price level
    pub orders: VecDeque<Order>,
    /// Total volume of all orders at this price level
    pub total_volume: Decimal,
}

impl PriceLevel {
    /// Returns the next order to be matched without removing it from the queue.
    /// This maintains FIFO ordering by always returning the front of the queue.
    ///
    /// # Returns
    /// * `Some(&Order)` - Reference to the next order to be matched
    /// * `None` - If there are no orders at this price level
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
    bids: BTreeMap<Decimal, PriceLevel>,
    /// Ask side orders organized by price (ascending)
    asks: BTreeMap<Decimal, PriceLevel>,
    /// Cache of best bid price for quick access
    best_bid: Option<Decimal>,
    /// Cache of best ask price for quick access
    best_ask: Option<Decimal>,
    /// Identifier for the instrument this order book manages
    instrument_id: Uuid,
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
        }
    }

    /// Adds a new order to the order book in price-time priority.
    /// Orders are organized first by price (best price first) and then by time of arrival (FIFO).
    ///
    /// # Arguments
    /// * `order` - The order to add to the book
    ///
    /// # Notes
    /// - Orders for different instruments are ignored
    /// - Market orders (no limit price) are ignored
    /// - Orders are added to the back of the queue at their price level
    /// - Best prices are automatically updated
    pub fn add_order(&mut self, order: Order) {
        // Verify this order is for our instrument
        if order.instrument_id != self.instrument_id {
            return;
        }

        // Get price from the order
        let price = match order.limit_price {
            Some(price) => price,
            None => return, // Can't add market orders to the book
        };

        let price_levels = match order.side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        // Get or create the price level
        let price_level = price_levels
            .entry(price)
            .or_insert_with(|| PriceLevel {
                price,
                orders: VecDeque::new(),
                total_volume: Decimal::ZERO,
            });

        // Add the order to the back of the queue (FIFO)
        price_level.orders.push_back(order.clone());
        price_level.total_volume += order.remaining_base;

        // Update best prices cache
        self.update_best_prices();
    }

    /// Removes an order from the order book.
    ///
    /// # Arguments
    /// * `order_id` - The unique identifier of the order to remove
    /// * `side` - The side (Bid/Ask) of the order
    /// * `price` - The price level of the order
    ///
    /// # Returns
    /// * `Some(Order)` - The removed order if found
    /// * `None` - If the order was not found
    ///
    /// # Notes
    /// - Maintains FIFO ordering of remaining orders
    /// - Updates total volume at the price level
    /// - Removes empty price levels
    /// - Updates best prices if necessary
    pub fn remove_order(&mut self, order_id: Uuid, side: Side, price: Decimal) -> Option<Order> {
        let price_levels = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        if let Some(price_level) = price_levels.get_mut(&price) {
            // Find and remove the order
            if let Some(pos) = price_level.orders.iter().position(|o| o.id == order_id) {
                // Use a safer approach to remove the order
                if let Some(order) = price_level.orders.remove(pos) {
                    price_level.total_volume -= order.remaining_base;

                    // If the price level is empty, remove it
                    if price_level.orders.is_empty() {
                        price_levels.remove(&price);
                    }

                    // Update best prices cache
                    self.update_best_prices();
                    return Some(order);
                }
            }
        }
        None
    }

    /// Returns the best order at a given side without removing it.
    ///
    /// # Arguments
    /// * `side` - The side (Bid/Ask) to peek at
    ///
    /// # Returns
    /// * `Some(&Order)` - Reference to the next order to be matched
    /// * `None` - If there are no orders on the specified side
    ///
    /// # Notes
    /// - For bids, returns the highest priced order
    /// - For asks, returns the lowest priced order
    /// - Within a price level, returns the first order (FIFO)
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
    pub fn get_orders_at_price(&self, side: Side, price: Decimal) -> Option<&VecDeque<Order>> {
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
    pub fn order_count_at_price(&self, side: Side, price: Decimal) -> usize {
        let price_levels = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };
        price_levels.get(&price).map_or(0, |level| level.order_count())
    }

    /// Updates the cached best prices for quick access.
    /// This is called automatically after order modifications.
    ///
    /// # Notes
    /// - For bids, sets to the highest price with orders
    /// - For asks, sets to the lowest price with orders
    /// - Sets to None if no orders exist on that side
    fn update_best_prices(&mut self) {
        // For buys, we want the highest price (last key in descending BTreeMap)
        self.best_bid = self.bids.keys().next_back().cloned();
        // For sells, we want the lowest price (first key in ascending BTreeMap)
        self.best_ask = self.asks.keys().next().cloned();
    }

    /// Returns the best bid price.
    ///
    /// # Returns
    /// * `Some(Decimal)` - The highest bid price with orders
    /// * `None` - If there are no bid orders
    pub fn best_bid(&self) -> Option<Decimal> {
        self.best_bid
    }

    /// Returns the best ask price.
    ///
    /// # Returns
    /// * `Some(Decimal)` - The lowest ask price with orders
    /// * `None` - If there are no ask orders
    pub fn best_ask(&self) -> Option<Decimal> {
        self.best_ask
    }

    /// Returns the spread between the best bid and ask prices.
    ///
    /// # Returns
    /// * `Some(Decimal)` - The difference between best ask and best bid
    /// * `None` - If either best bid or best ask is missing
    pub fn spread(&self) -> Option<Decimal> {
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
    /// * `Some(Decimal)` - The total volume at the specified price
    /// * `None` - If no orders exist at the specified price
    pub fn volume_at_price(&self, side: Side, price: Decimal) -> Option<Decimal> {
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
    pub fn get_best_bid(&self) -> Option<&Order> {
        self.peek_best_order(Side::Bid)
    }

    /// Gets the best ask order without removing it from the book.
    ///
    /// # Returns
    /// * `Some(&Order)` - Reference to the best ask order (lowest price)
    /// * `None` - If there are no ask orders
    pub fn get_best_ask(&self) -> Option<&Order> {
        self.peek_best_order(Side::Ask)
    }
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
    use rust_decimal_macros::dec;
    use crate::types::{OrderType, CreatedFrom};
    use chrono::Utc;

    /// Creates a test order with the specified parameters.
    ///
    /// # Arguments
    /// * `side` - The side (Bid/Ask) of the order
    /// * `price` - The limit price of the order
    /// * `quantity` - The quantity/size of the order
    /// * `instrument_id` - The instrument identifier
    ///
    /// # Returns
    /// A new Order instance with default values for other fields
    fn create_test_order(side: Side, price: Decimal, quantity: Decimal, instrument_id: Uuid) -> Order {
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
            filled_quote: dec!(0.0),
            filled_base: dec!(0.0),
            remaining_quote: price * quantity,
            expiration_date: now + chrono::Duration::days(365),
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 1,
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
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), None);
        assert_eq!(book.volume_at_price(Side::Ask, dec!(100.0)), None);
    }

    /// Tests basic operations with a single order.
    #[test]
    fn test_single_order() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
        book.add_order(order.clone());
        
        assert_eq!(book.best_bid(), Some(dec!(100.0)));
        assert_eq!(book.best_ask(), None);
        assert_eq!(book.spread(), None);
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), Some(dec!(1.0)));
    }

    /// Tests handling of multiple orders at the same price level.
    #[test]
    fn test_multiple_orders_same_price() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add multiple orders at same price
        for _ in 0..5 {
            let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
            book.add_order(order);
        }
        
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), Some(dec!(5.0)));
    }

    /// Tests order management across different price levels.
    #[test]
    fn test_price_levels() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add orders at different price levels
        let prices = [dec!(100.0), dec!(99.0), dec!(101.0)];
        for price in prices {
            let order = create_test_order(Side::Bid, price, dec!(1.0), instrument_id);
            book.add_order(order);
        }
        
        assert_eq!(book.best_bid(), Some(dec!(101.0))); // Highest bid
    }

    /// Tests order removal functionality.
    #[test]
    fn test_remove_order() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
        book.add_order(order.clone());
        
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), Some(dec!(1.0)));
        
        let limit_price = match order.limit_price {
            Some(price) => price,
            None => panic!("Expected order to have a limit price"),
        };
        let removed = book.remove_order(order.id, order.side, limit_price);
        assert!(removed.is_some());
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), None);
    }

    /// Tests handling of non-existent order removal.
    #[test]
    fn test_remove_nonexistent_order() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
        book.add_order(order.clone());
        
        // Try to remove with wrong price
        let removed = book.remove_order(order.id, order.side, dec!(99.0));
        assert!(removed.is_none());
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), Some(dec!(1.0)));
    }

    /// Tests spread calculation between bid and ask sides.
    #[test]
    fn test_spread_calculation() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add bid and ask orders
        let bid_order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
        let ask_order = create_test_order(Side::Ask, dec!(101.0), dec!(1.0), instrument_id);
        
        book.add_order(bid_order);
        book.add_order(ask_order);
        
        assert_eq!(book.spread(), Some(dec!(1.0)));
    }

    /// Tests handling of orders for wrong instrument IDs.
    #[test]
    fn test_wrong_instrument_id() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Try to add order for different instrument
        let wrong_instrument_id = Uuid::new_v4();
        let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), wrong_instrument_id);
        
        book.add_order(order);
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), None);
    }

    /// Tests volume tracking across multiple price levels.
    #[test]
    fn test_multiple_price_levels_volume() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add orders at different price levels
        let price_levels = [
            (dec!(100.0), dec!(2.0)),
            (dec!(99.0), dec!(3.0)),
            (dec!(101.0), dec!(1.0)),
        ];
        
        for (price, quantity) in price_levels {
            let order = create_test_order(Side::Bid, price, quantity, instrument_id);
            book.add_order(order);
        }
        
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), Some(dec!(2.0)));
        assert_eq!(book.volume_at_price(Side::Bid, dec!(99.0)), Some(dec!(3.0)));
        assert_eq!(book.volume_at_price(Side::Bid, dec!(101.0)), Some(dec!(1.0)));
    }

    /// Tests FIFO ordering of orders within price levels.
    #[test]
    fn test_fifo_order_execution() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add orders with different sequence IDs but same price
        let mut orders = Vec::new();
        for i in 1..=3 {
            let mut order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
            order.sequence_id = i;
            orders.push(order.clone());
            book.add_order(order);
        }

        // Verify get_best_bid returns the first order
        let best_order = match book.get_best_bid() {
            Some(order) => order,
            None => panic!("Expected to find a best bid order"),
        };
        assert_eq!(best_order.sequence_id, 1);

        // Remove first order and verify next one becomes best
        book.remove_order(orders[0].id, Side::Bid, dec!(100.0));
        let next_best = match book.get_best_bid() {
            Some(order) => order,
            None => panic!("Expected to find a next best bid order"),
        };
        assert_eq!(next_best.sequence_id, 2);
    }

    /// Tests order counting functionality at price levels.
    #[test]
    fn test_order_count_tracking() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add multiple orders at same price
        for _ in 0..3 {
            let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
            book.add_order(order);
        }

        assert_eq!(book.order_count_at_price(Side::Bid, dec!(100.0)), 3);
        
        // Add orders at different price
        let order = create_test_order(Side::Bid, dec!(101.0), dec!(1.0), instrument_id);
        book.add_order(order);
        
        assert_eq!(book.order_count_at_price(Side::Bid, dec!(101.0)), 1);
    }

    /// Tests various edge cases in order handling.
    #[test]
    fn test_edge_cases() {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Test zero quantity
        let zero_order = create_test_order(Side::Bid, dec!(100.0), dec!(0.0), instrument_id);
        book.add_order(zero_order);
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), Some(dec!(0.0)));
        
        // Test very large quantity
        let large_order = create_test_order(Side::Bid, dec!(100.0), dec!(1_000_000.0), instrument_id);
        book.add_order(large_order);
        assert_eq!(book.volume_at_price(Side::Bid, dec!(100.0)), Some(dec!(1_000_000.0)));
        
        // Test very small price
        let small_price_order = create_test_order(Side::Bid, dec!(0.000001), dec!(1.0), instrument_id);
        book.add_order(small_price_order);
        assert_eq!(book.volume_at_price(Side::Bid, dec!(0.000001)), Some(dec!(1.0)));
    }
}
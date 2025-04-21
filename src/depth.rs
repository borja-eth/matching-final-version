//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements a high-performance order book depth tracker.
// It maintains cached aggregated views of the order book to provide fast access to depth information.
//
// | Component                | Description                                                |
// |--------------------------|-----------------------------------------------------------|
// | Price                    | Newtype wrapper for price with strong typing               |
// | PriceLevel              | Aggregated volume information at a specific price          |
// | DepthSnapshot           | Immutable point-in-time view of order book depth           |
// | DepthTracker            | Real-time tracker of order book depth with fast updates    |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | Price                   | Strongly typed price wrapper                      | new, inner        |
// | PriceLevel              | Price level with aggregated data                  | volume, count     |
// | DepthSnapshot           | Immutable depth snapshot                          | bids, asks        |
// | DepthTracker            | Main depth tracking component                     | update, snapshot  |
//
//--------------------------------------------------------------------------------------------------
// FUNCTIONS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Return Type      |
// |-------------------------|---------------------------------------------------|------------------|
// | new                     | Constructs a new DepthTracker                     | DepthTracker     |
// | update_order_added      | Updates depth on order addition                   | ()               |
// | update_order_removed    | Updates depth on order removal                    | ()               |
// | update_order_matched    | Updates depth on order match                      | ()               |
// | get_snapshot            | Creates snapshot of current depth                 | DepthSnapshot    |
//--------------------------------------------------------------------------------------------------

use std::collections::{BTreeMap, btree_map::Entry};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::types::{Order, Side};

/// Newtype wrapper for price to provide type safety and semantic meaning
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Price(i64);

impl Price {
    /// Creates a new Price from an i64
    #[inline]
    pub fn new(price: i64) -> Self {
        Self(price)
    }

    /// Gets the inner i64 value
    #[inline]
    pub fn inner(&self) -> i64 {
        self.0
    }
}

/// Represents an aggregated price level in the depth view
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PriceLevel {
    /// The price for this level
    pub price: i64,
    /// Total volume at this price level
    pub volume: u64,
    /// Number of orders at this price level
    pub order_count: u32,
}

impl PriceLevel {
    /// Creates a new price level
    #[inline]
    pub fn new(price: i64, volume: u64, order_count: u32) -> Self {
        Self {
            price,
            volume,
            order_count,
        }
    }

    /// Creates a price level from an initial order
    #[inline]
    pub fn from_order(order: &Order) -> Option<Self> {
        order.limit_price.map(|price| Self {
            price,
            volume: order.remaining_base,
            order_count: 1,
        })
    }
}

/// An immutable snapshot of order book depth at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthSnapshot {
    /// Bid price levels ordered by price descending (best bids first)
    pub bids: Vec<PriceLevel>,
    /// Ask price levels ordered by price ascending (best asks first)
    pub asks: Vec<PriceLevel>,
    /// Timestamp when this snapshot was taken
    pub timestamp: DateTime<Utc>,
    /// Instrument ID this depth snapshot belongs to
    pub instrument_id: Uuid,
}

impl DepthSnapshot {
    /// Creates a new depth snapshot
    #[inline]
    pub fn new(bids: Vec<PriceLevel>, asks: Vec<PriceLevel>, instrument_id: Uuid) -> Self {
        Self {
            bids,
            asks,
            timestamp: Utc::now(),
            instrument_id,
        }
    }

    /// Returns the best bid price if available
    #[inline]
    pub fn best_bid(&self) -> Option<i64> {
        self.bids.first().map(|level| level.price)
    }

    /// Returns the best ask price if available
    #[inline]
    pub fn best_ask(&self) -> Option<i64> {
        self.asks.first().map(|level| level.price)
    }

    /// Returns the current spread (best ask - best bid)
    #[inline]
    pub fn spread(&self) -> Option<i64> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }
}

/// Main component for tracking depth information in real-time
#[derive(Debug)]
pub struct DepthTracker {
    /// Instrument ID this depth tracker belongs to
    instrument_id: Uuid,
    /// Bid side price levels ordered by price (BTreeMap will sort keys automatically)
    bids: BTreeMap<Price, PriceLevel>,
    /// Ask side price levels ordered by price (BTreeMap will sort keys automatically)
    asks: BTreeMap<Price, PriceLevel>,
    /// Pre-allocated buffer for creating bid snapshots to avoid allocations
    bids_snapshot_buffer: Vec<PriceLevel>,
    /// Pre-allocated buffer for creating ask snapshots to avoid allocations
    asks_snapshot_buffer: Vec<PriceLevel>,
}

impl DepthTracker {
    /// Creates a new depth tracker for a specific instrument
    pub fn new(instrument_id: Uuid) -> Self {
        // Pre-allocate with a reasonable capacity to avoid early reallocations
        const DEFAULT_DEPTH_CAPACITY: usize = 20;
        
        Self {
            instrument_id,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            bids_snapshot_buffer: Vec::with_capacity(DEFAULT_DEPTH_CAPACITY),
            asks_snapshot_buffer: Vec::with_capacity(DEFAULT_DEPTH_CAPACITY),
        }
    }

    /// Updates depth when an order is added to the book
    #[inline]
    pub fn update_order_added(&mut self, order: &Order) {
        if let Some(price) = order.limit_price {
            let price_key = Price::new(price);
            let price_levels = match order.side {
                Side::Bid => &mut self.bids,
                Side::Ask => &mut self.asks,
            };
            
            // Update the price level using entry API for efficient upsert
            match price_levels.entry(price_key) {
                Entry::Vacant(entry) => {
                    entry.insert(PriceLevel {
                        price,
                        volume: order.remaining_base,
                        order_count: 1,
                    });
                }
                Entry::Occupied(mut entry) => {
                    let level = entry.get_mut();
                    level.volume += order.remaining_base;
                    level.order_count += 1;
                }
            }
        }
    }

    /// Updates depth when an order is removed from the book
    #[inline]
    pub fn update_order_removed(&mut self, order: &Order) {
        if let Some(price) = order.limit_price {
            let price_key = Price::new(price);
            let price_levels = match order.side {
                Side::Bid => &mut self.bids,
                Side::Ask => &mut self.asks,
            };

            // Find the price level and update or remove it
            if let Entry::Occupied(mut entry) = price_levels.entry(price_key) {
                let level = entry.get_mut();
                
                // Subtract volume and decrement count
                level.volume -= order.remaining_base;
                level.order_count = level.order_count.saturating_sub(1);
                
                // If no orders left at this level, remove it
                if level.order_count == 0 || level.volume == 0 {
                    entry.remove();
                }
            }
        }
    }

    /// Updates depth when an order is matched (partial or full fill)
    #[inline]
    pub fn update_order_matched(&mut self, order: &Order, matched_quantity: u64) {
        if let Some(price) = order.limit_price {
            let price_key = Price::new(price);
            let price_levels = match order.side {
                Side::Bid => &mut self.bids,
                Side::Ask => &mut self.asks,
            };

            // Find the price level and update it
            if let Some(level) = price_levels.get_mut(&price_key) {
                // Subtract matched quantity
                level.volume = level.volume.saturating_sub(matched_quantity);
                
                // If fully filled, decrement order count
                if order.remaining_base == 0 {
                    level.order_count = level.order_count.saturating_sub(1);
                    
                    // If no orders left at this level, remove it
                    if level.order_count == 0 || level.volume == 0 {
                        price_levels.remove(&price_key);
                    }
                }
            }
        }
    }

    /// Gets a snapshot of the current depth with a specified limit of levels per side
    #[inline]
    pub fn get_snapshot(&mut self, limit: usize) -> DepthSnapshot {
        // Clear reusable buffers
        self.bids_snapshot_buffer.clear();
        self.asks_snapshot_buffer.clear();
        
        // Ensure buffers have enough capacity
        if self.bids_snapshot_buffer.capacity() < limit {
            self.bids_snapshot_buffer.reserve(limit - self.bids_snapshot_buffer.capacity());
        }
        if self.asks_snapshot_buffer.capacity() < limit {
            self.asks_snapshot_buffer.reserve(limit - self.asks_snapshot_buffer.capacity());
        }
        
        // Fill bid buffer (in reverse order for descending prices)
        for level in self.bids.values().rev().take(limit) {
            self.bids_snapshot_buffer.push(*level);
        }
        
        // Fill ask buffer
        for level in self.asks.values().take(limit) {
            self.asks_snapshot_buffer.push(*level);
        }
        
        // Create snapshot using buffers
        DepthSnapshot::new(
            self.bids_snapshot_buffer.clone(), 
            self.asks_snapshot_buffer.clone(),
            self.instrument_id
        )
    }

    /// Gets a snapshot without allocation by writing into provided vectors
    #[inline]
    pub fn get_snapshot_into(&self, bids: &mut Vec<PriceLevel>, asks: &mut Vec<PriceLevel>, limit: usize) {
        // Clear output vectors
        bids.clear();
        asks.clear();
        
        // Ensure vectors have enough capacity
        if bids.capacity() < limit {
            bids.reserve(limit - bids.capacity());
        }
        if asks.capacity() < limit {
            asks.reserve(limit - asks.capacity());
        }
        
        // Fill bid buffer (in reverse order for descending prices)
        for level in self.bids.values().rev().take(limit) {
            bids.push(*level);
        }
        
        // Fill ask buffer
        for level in self.asks.values().take(limit) {
            asks.push(*level);
        }
    }

    /// Returns the instrument ID this depth tracker belongs to
    #[inline]
    pub fn instrument_id(&self) -> Uuid {
        self.instrument_id
    }

    /// Returns the total number of price levels on the bid side
    #[inline]
    pub fn bid_level_count(&self) -> usize {
        self.bids.len()
    }

    /// Returns the total number of price levels on the ask side
    #[inline]
    pub fn ask_level_count(&self) -> usize {
        self.asks.len()
    }

    /// Returns the best bid price level if available
    #[inline]
    pub fn best_bid(&self) -> Option<PriceLevel> {
        self.bids.values().rev().next().copied()
    }

    /// Returns the best ask price level if available
    #[inline]
    pub fn best_ask(&self) -> Option<PriceLevel> {
        self.asks.values().next().copied()
    }

    /// Returns the total volume across all bid levels
    #[inline]
    pub fn total_bid_volume(&self) -> u64 {
        self.bids.values().map(|level| level.volume).sum()
    }

    /// Returns the total volume across all ask levels
    #[inline]
    pub fn total_ask_volume(&self) -> u64 {
        self.asks.values().map(|level| level.volume).sum()
    }
}

/// Implements a thread-safe shared depth tracker that can be used across threads
pub struct SharedDepthTracker(Arc<parking_lot::RwLock<DepthTracker>>);

impl SharedDepthTracker {
    /// Creates a new shared depth tracker
    pub fn new(instrument_id: Uuid) -> Self {
        Self(Arc::new(parking_lot::RwLock::new(DepthTracker::new(instrument_id))))
    }

    /// Updates depth when an order is added (acquires write lock)
    pub fn update_order_added(&self, order: &Order) {
        let mut tracker = self.0.write();
        tracker.update_order_added(order);
    }

    /// Updates depth when an order is removed (acquires write lock)
    pub fn update_order_removed(&self, order: &Order) {
        let mut tracker = self.0.write();
        tracker.update_order_removed(order);
    }

    /// Updates depth when an order is matched (acquires write lock)
    pub fn update_order_matched(&self, order: &Order, matched_quantity: u64) {
        let mut tracker = self.0.write();
        tracker.update_order_matched(order, matched_quantity);
    }

    /// Gets a snapshot of the current depth (acquires read lock)
    pub fn get_snapshot(&self, limit: usize) -> DepthSnapshot {
        let mut tracker = self.0.write(); // Need write lock for internal buffer management
        tracker.get_snapshot(limit)
    }

    /// Gets the instrument ID this depth tracker belongs to (acquires read lock)
    pub fn instrument_id(&self) -> Uuid {
        let tracker = self.0.read();
        tracker.instrument_id()
    }

    /// Clones the shared tracker, increasing the reference count
    pub fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderType, OrderStatus, CreatedFrom, TimeInForce};
    
    // Helper to create test orders
    fn create_test_order(
        side: Side, 
        price: i64, 
        quantity: u64,
        instrument_id: Uuid
    ) -> Order {
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
            remaining_quote: (price as u64) * quantity,
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
    
    #[test]
    fn test_empty_depth_tracker() {
        let instrument_id = Uuid::new_v4();
        let mut tracker = DepthTracker::new(instrument_id);
        
        // Test empty state
        assert_eq!(tracker.bid_level_count(), 0);
        assert_eq!(tracker.ask_level_count(), 0);
        assert_eq!(tracker.best_bid(), None);
        assert_eq!(tracker.best_ask(), None);
        
        // Test empty snapshot
        let snapshot = tracker.get_snapshot(10);
        assert!(snapshot.bids.is_empty());
        assert!(snapshot.asks.is_empty());
        assert_eq!(snapshot.instrument_id, instrument_id);
    }
    
    #[test]
    fn test_add_orders() {
        let instrument_id = Uuid::new_v4();
        let mut tracker = DepthTracker::new(instrument_id);
        
        // Add bid order
        let bid_order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        tracker.update_order_added(&bid_order);
        
        // Verify bid side
        assert_eq!(tracker.bid_level_count(), 1);
        assert_eq!(tracker.best_bid().unwrap().price, 100_000);
        assert_eq!(tracker.best_bid().unwrap().volume, 100_000);
        assert_eq!(tracker.best_bid().unwrap().order_count, 1);
        
        // Add ask order
        let ask_order = create_test_order(Side::Ask, 101_000, 200_000, instrument_id);
        tracker.update_order_added(&ask_order);
        
        // Verify ask side
        assert_eq!(tracker.ask_level_count(), 1);
        assert_eq!(tracker.best_ask().unwrap().price, 101_000);
        assert_eq!(tracker.best_ask().unwrap().volume, 200_000);
        assert_eq!(tracker.best_ask().unwrap().order_count, 1);
        
        // Get snapshot
        let snapshot = tracker.get_snapshot(10);
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.asks.len(), 1);
        assert_eq!(snapshot.best_bid(), Some(100_000));
        assert_eq!(snapshot.best_ask(), Some(101_000));
        assert_eq!(snapshot.spread(), Some(1_000));
    }
    
    #[test]
    fn test_add_multiple_orders_same_price() {
        let instrument_id = Uuid::new_v4();
        let mut tracker = DepthTracker::new(instrument_id);
        
        // Add multiple bid orders at same price
        let bid_order1 = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        let bid_order2 = create_test_order(Side::Bid, 100_000, 200_000, instrument_id);
        
        tracker.update_order_added(&bid_order1);
        tracker.update_order_added(&bid_order2);
        
        // Verify aggregated level
        assert_eq!(tracker.bid_level_count(), 1);
        assert_eq!(tracker.best_bid().unwrap().volume, 300_000);
        assert_eq!(tracker.best_bid().unwrap().order_count, 2);
    }
    
    #[test]
    fn test_add_multiple_price_levels() {
        let instrument_id = Uuid::new_v4();
        let mut tracker = DepthTracker::new(instrument_id);
        
        // Add bids at different prices
        let bid_orders = [
            create_test_order(Side::Bid, 100_000, 100_000, instrument_id),
            create_test_order(Side::Bid, 99_000, 200_000, instrument_id),
            create_test_order(Side::Bid, 101_000, 50_000, instrument_id),
        ];
        
        for order in &bid_orders {
            tracker.update_order_added(order);
        }
        
        // Verify multiple levels
        assert_eq!(tracker.bid_level_count(), 3);
        assert_eq!(tracker.best_bid().unwrap().price, 101_000);
        
        // Get snapshot and verify ordering (descending for bids)
        let snapshot = tracker.get_snapshot(10);
        assert_eq!(snapshot.bids.len(), 3);
        assert_eq!(snapshot.bids[0].price, 101_000);
        assert_eq!(snapshot.bids[1].price, 100_000);
        assert_eq!(snapshot.bids[2].price, 99_000);
    }
    
    #[test]
    fn test_remove_orders() {
        let instrument_id = Uuid::new_v4();
        let mut tracker = DepthTracker::new(instrument_id);
        
        // Add bid order
        let bid_order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        tracker.update_order_added(&bid_order);
        
        // Remove it
        tracker.update_order_removed(&bid_order);
        
        // Verify removal
        assert_eq!(tracker.bid_level_count(), 0);
        assert_eq!(tracker.best_bid(), None);
    }
    
    #[test]
    fn test_partial_fill() {
        let instrument_id = Uuid::new_v4();
        let mut tracker = DepthTracker::new(instrument_id);
        
        // Add bid order
        let mut bid_order = create_test_order(Side::Bid, 100_000, 200_000, instrument_id);
        tracker.update_order_added(&bid_order);
        
        // Partially fill order
        let filled_qty = 50_000;
        bid_order.remaining_base = bid_order.remaining_base.saturating_sub(filled_qty);
        
        tracker.update_order_matched(&bid_order, filled_qty);
        
        // Verify partial fill
        assert_eq!(tracker.best_bid().unwrap().volume, 150_000);
        assert_eq!(tracker.best_bid().unwrap().order_count, 1);
    }
    
    #[test]
    fn test_full_fill() {
        let instrument_id = Uuid::new_v4();
        let mut tracker = DepthTracker::new(instrument_id);
        
        // Add bid order
        let mut bid_order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        tracker.update_order_added(&bid_order);
        
        // Fully fill order
        let filled_qty = 100_000;
        bid_order.remaining_base = 0;
        
        tracker.update_order_matched(&bid_order, filled_qty);
        
        // Verify level was removed
        assert_eq!(tracker.bid_level_count(), 0);
        assert_eq!(tracker.best_bid(), None);
    }
    
    #[test]
    fn test_snapshot_with_limit() {
        let instrument_id = Uuid::new_v4();
        let mut tracker = DepthTracker::new(instrument_id);
        
        // Add multiple price levels
        for i in 1..=5 {
            let price = 100_000 + i * 1_000;
            let order = create_test_order(Side::Ask, price, (100_000 + i * 1_000) as u64, instrument_id);
            tracker.update_order_added(&order);
        }
        
        // Get limited snapshot
        let snapshot = tracker.get_snapshot(3);
        
        // Verify only 3 levels are returned
        assert_eq!(snapshot.asks.len(), 3);
        assert_eq!(snapshot.asks[0].price, 101_000);
        assert_eq!(snapshot.asks[1].price, 102_000);
        assert_eq!(snapshot.asks[2].price, 103_000);
    }
    
    #[test]
    fn test_get_snapshot_into() {
        let instrument_id = Uuid::new_v4();
        let tracker = DepthTracker::new(instrument_id);
        
        // Pre-allocate buffers
        let mut bids = Vec::with_capacity(10);
        let mut asks = Vec::with_capacity(10);
        
        // Get snapshot into buffers
        tracker.get_snapshot_into(&mut bids, &mut asks, 10);
        
        // Verify empty result doesn't allocate
        assert!(bids.is_empty());
        assert!(asks.is_empty());
        assert_eq!(bids.capacity(), 10);
        assert_eq!(asks.capacity(), 10);
    }
    
    #[test]
    fn test_shared_depth_tracker() {
        let instrument_id = Uuid::new_v4();
        let shared_tracker = SharedDepthTracker::new(instrument_id);
        
        // Add bid order
        let bid_order = create_test_order(Side::Bid, 100_000, 100_000, instrument_id);
        shared_tracker.update_order_added(&bid_order);
        
        // Get snapshot
        let snapshot = shared_tracker.get_snapshot(10);
        
        // Verify order was added
        assert_eq!(snapshot.bids.len(), 1);
        assert_eq!(snapshot.bids[0].price, 100_000);
        
        // Clone the tracker
        let tracker_clone = shared_tracker.clone();
        
        // Add through clone
        let ask_order = create_test_order(Side::Ask, 101_000, 200_000, instrument_id);
        tracker_clone.update_order_added(&ask_order);
        
        // Get snapshot from original
        let updated_snapshot = shared_tracker.get_snapshot(10);
        
        // Verify both trackers share the same state
        assert_eq!(updated_snapshot.asks.len(), 1);
        assert_eq!(updated_snapshot.asks[0].price, 101_000);
    }
} 
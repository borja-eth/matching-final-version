# Simplified Matching Engine Architecture

## Overview

This document outlines a simplified architecture for a matching engine implemented in Rust. This design prioritizes simplicity, clarity, and robustness while maintaining high performance. This approach employs a central orchestrator (Matching Engine) to manage the order book and depth tracking for a single instrument, with the ability to scale later.

## System Flow

```
                        SIMPLIFIED HIGH-LEVEL FLOW
┌────────────────┐     ┌────────────────┐     ┌────────────────┐
│                │     │                │     │                │
│  API Gateway   │────▶│  Matching      │────▶│  Event Stream  │
│                │     │  Engine        │     │                │
└────────────────┘     └────────────────┘     └────────────────┘
                             │   ▲
                             │   │
                             ▼   │
                       ┌────────────────┐
                       │                │
                       │  Order Book    │
                       │                │
                       └────────────────┘
                             │   ▲
                             │   │
                             ▼   │
                       ┌────────────────┐
                       │                │
                       │  Depth Module  │
                       │                │
                       └────────────────┘
```

## MVP (Minimum Viable Product)

### Core Components

For a minimal viable product, we'll focus on three essential components:

1. **OrderBook**: Manages the limit order book for a single instrument
2. **MatchingEngine**: Processes incoming orders and executes matching logic
3. **DepthModule**: Maintains aggregated market depth information

### Minimal Feature Set

#### Order Types
- **GTC (Good Till Cancelled)** - Standard limit orders that remain on the book until filled or cancelled
- **IOC (Immediate or Cancel)** - Orders that execute immediately and cancel any unfilled portion
- **Market** - Treated directly as IOC orders that accept any price

#### Order Operations
- Add new order
- Cancel existing order
- Match incoming orders against the book
- Generate trade events
- Update market depth

### MVP Implementation Structure

```rust
/// Manages the storage and matching logic for orders
pub struct OrderBook {
    /// Buy side of the book (price descending)
    buy_levels: BTreeMap<Decimal, PriceLevel>,
    /// Sell side of the book (price ascending)
    sell_levels: BTreeMap<Decimal, PriceLevel>,
    /// Map of active orders for fast lookups
    orders: HashMap<Uuid, OrderRef>,
    /// Last traded price
    last_trade_price: Option<Decimal>,
}

/// Central coordinator for order processing
pub struct MatchingEngine {
    /// The primary order book
    order_book: OrderBook,
    /// Market depth information
    depth_module: DepthModule,
    /// Event emitter for trade and order events
    event_emitter: EventEmitter,
}

/// Maintains aggregated market depth information
pub struct DepthModule {
    /// Buy side depth (price → volume)
    buy_depth: BTreeMap<Decimal, Decimal>,
    /// Sell side depth (price → volume)
    sell_depth: BTreeMap<Decimal, Decimal>,
    /// Number of levels to track (configurable)
    max_depth: usize,
}
```

## Core Order Flow

### Simplified Order Lifecycle

```
┌──────────────────┐
│                  │
│   New Order      │
│                  │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│                  │
│   Validation     │
│                  │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│                  │
│  Attempt Match   │
│                  │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐    Yes   ┌──────────────────┐
│                  │─────────▶│                  │
│  Is IOC/Market?  │          │  Cancel Remainder│
│                  │          │                  │
└────────┬─────────┘          └──────────────────┘
         │ No                   
         ▼                      
┌──────────────────┐           
│                  │           
│  Add to Book     │           
│  (GTC only)      │           
│                  │           
└────────┬─────────┘           
         │                     
         ▼                     
┌──────────────────┐           
│                  │           
│  Update Depth    │           
│                  │           
└──────────────────┘           
```

### Function-Level Flow 

#### Regular Limit Order (GTC)

```rust
// In MatchingEngine
pub fn process_limit_order(&mut self, order: Order) -> Result<Vec<Trade>, MatchingError> {
    // Try to match the order with existing orders
    let (trades, remaining_order) = self.order_book.match_order(order)?;
    
    // If there's remaining quantity, add to the book (for GTC)
    if !order.is_ioc() && !order.is_market() {
        if let Some(remaining) = remaining_order {
            self.order_book.add_order(remaining)?;
            self.depth_module.update_after_order_added(&remaining);
        }
    } else if let Some(mut remaining) = remaining_order {
        // For IOC, cancel any remaining quantity
        remaining.status = OrderStatus::Cancelled;
        self.event_emitter.emit_order_cancelled(&remaining);
    }
    
    // Emit events for trades and update depth
    for trade in &trades {
        self.event_emitter.emit_trade(trade);
        self.depth_module.update_after_trade(trade);
    }
    
    Ok(trades)
}
```

#### Order Book Matching

```rust
// In OrderBook
pub fn match_order(&mut self, mut order: Order) -> Result<(Vec<Trade>, Option<Order>), MatchingError> {
    let mut trades = Vec::new();
    
    // Determine which side of the book to match against
    let book_side = match order.side {
        Side::Buy => &mut self.sell_levels,
        Side::Sell => &mut self.buy_levels,
    };
    
    // For a buy order, match against sell orders (and vice versa)
    // Continue matching until order is filled or no matching price levels remain
    while !is_order_complete(&order) {
        // Get the best price level (lowest sell for buy orders, highest buy for sell orders)
        let best_price_level = match book_side.first_entry() {
            Some(entry) => entry,
            None => break, // No more orders to match against
        };
        
        // For limit orders, check if the price is acceptable
        if !order.is_market() && !is_price_acceptable(&order, best_price_level.key()) {
            break; // Price not acceptable, stop matching
        }
        
        // Match against orders at this price level
        match_at_price_level(&mut order, best_price_level, &mut trades)?;
    }
    
    // Update last trade price if any trades occurred
    if !trades.is_empty() {
        self.last_trade_price = Some(trades.last().unwrap().price);
    }
    
    // Return trades and the remaining order (if any)
    if is_order_complete(&order) {
        Ok((trades, None))
    } else {
        Ok((trades, Some(order)))
    }
}
```

## Optimized Order Book Implementation

For exceptional performance while maintaining simplicity, we can optimize the order book data structures and access patterns:

### Improved Order Book Structure

```rust
/// An optimized order book implementation for high performance
pub struct OrderBook {
    /// Buy side of the book (price descending)
    /// Using BTreeMap for O(log n) price level lookup with natural ordering
    buy_levels: BTreeMap<Decimal, PriceLevel>,
    
    /// Sell side of the book (price ascending)
    /// Using BTreeMap for O(log n) price level lookup with natural ordering
    sell_levels: BTreeMap<Decimal, PriceLevel>,
    
    /// Fast lookup of orders by ID
    /// Using a simple HashMap for O(1) retrievals
    orders: HashMap<Uuid, OrderRef>,
    
    /// Cache of best bid/ask prices for instant access
    /// Avoiding tree traversal for common operations
    best_bid: Option<Decimal>,
    best_ask: Option<Decimal>,
    
    /// Last traded price
    last_trade_price: Option<Decimal>,
}

/// Represents a collection of orders at a specific price level
pub struct PriceLevel {
    /// The price of this level
    price: Decimal,
    
    /// Orders at this price level in FIFO order (time priority)
    /// VecDeque provides O(1) operations at both ends
    orders: VecDeque<Order>,
    
    /// Pre-calculated total volume at this level
    /// Avoids recalculating this frequently accessed value
    total_volume: Decimal,
}
```

### Key Optimizations

1. **Price Level Caching**
   ```rust
   impl OrderBook {
       /// Updates the cached best bid/ask after any book modification
       fn update_best_prices(&mut self) {
           self.best_bid = self.buy_levels.keys().next().cloned();
           self.best_ask = self.sell_levels.keys().next().cloned();
       }
       
       /// Get best bid price with O(1) complexity
       pub fn best_bid(&self) -> Option<Decimal> {
           self.best_bid
       }
       
       /// Get best ask price with O(1) complexity
       pub fn best_ask(&self) -> Option<Decimal> {
           self.best_ask
       }
   }
   ```

2. **Pre-allocated Memory**
   ```rust
   impl PriceLevel {
       /// Create a new price level with pre-allocated capacity
       pub fn with_capacity(price: Decimal, capacity: usize) -> Self {
           Self {
               price,
               orders: VecDeque::with_capacity(capacity),
               total_volume: Decimal::ZERO,
           }
       }
   }
   
   impl OrderBook {
       /// Add an order with capacity hint
       pub fn add_order(&mut self, order: Order, expected_level_size: usize) -> Result<(), MatchingError> {
           // ... existing code ...
           
           // Get or create price level with capacity hint
           let price_level = book_side
               .entry(price)
               .or_insert_with(|| PriceLevel::with_capacity(price, expected_level_size));
               
           // ... existing code ...
       }
   }
   ```

3. **Optimized Matching Loop**
   ```rust
   /// Special fast path for market orders that will take any price
   /// This avoids repetitive price acceptance checks in the hot path
   pub fn match_market_order(&mut self, mut order: Order) -> Result<(Vec<Trade>, Option<Order>), MatchingError> {
       let mut trades = Vec::with_capacity(10); // Pre-allocate for common case
       
       // Direct book side reference to avoid match in the loop
       let book_side = if order.side == Side::Buy {
           &mut self.sell_levels
       } else {
           &mut self.buy_levels
       };
       
       // Fast market order matching without price checks
       while !is_order_complete(&order) {
           if let Some(mut entry) = book_side.first_entry() {
               match_at_price_level(&mut order, &mut entry, &mut trades)?;
               
               // Remove the price level if it's empty
               if entry.get().orders.is_empty() {
                   book_side.pop_first();
               }
           } else {
               break; // No more orders to match
           }
       }
       
       // ... update best prices cache ...
       self.update_best_prices();
       
       // Return result
       if is_order_complete(&order) {
           Ok((trades, None))
       } else {
           Ok((trades, Some(order)))
       }
   }
   ```

4. **Fixed-Size Price Increment Optimization**
   ```rust
   /// For markets with fixed tick sizes, use integer multiples of tick size as keys
   /// This improves BTreeMap performance by using integers instead of Decimal
   pub struct FixedTickOrderBook {
       /// Tick size (e.g., 0.01 for penny increments)
       tick_size: Decimal,
       
       /// Buy side using integer price level multiples
       /// e.g., price level 42 = 42 * tick_size
       buy_levels: BTreeMap<i64, PriceLevel>,
       
       /// Sell side using integer price level multiples
       sell_levels: BTreeMap<i64, PriceLevel>,
       
       // ... other fields ...
   }
   
   impl FixedTickOrderBook {
       /// Convert decimal price to tick multiple
       fn price_to_ticks(&self, price: Decimal) -> i64 {
           (price / self.tick_size).to_i64().unwrap()
       }
       
       /// Convert tick multiple back to decimal price
       fn ticks_to_price(&self, ticks: i64) -> Decimal {
           Decimal::from(ticks) * self.tick_size
       }
   }
   ```

### Minimalist Concurrency Approach

For the MVP, we can use a simple but effective concurrency model:

```rust
/// Thread-safe order book that minimizes locking contention
pub struct ConcurrentOrderBook {
    /// The core order book protected by a single mutex
    /// Simple, predictable locking behavior
    inner: Mutex<OrderBook>,
    
    /// Statistics that can be updated without locking the book
    stats: OrderBookStats,
}

/// Statistics that can be updated atomically without locking the book
pub struct OrderBookStats {
    /// Count of orders in the book
    order_count: AtomicU64,
    
    /// Count of completed trades
    trade_count: AtomicU64,
    
    /// Last traded price (atomic reference)
    last_price: AtomicRef<Option<Decimal>>,
}

impl ConcurrentOrderBook {
    /// Process an order with full locking
    pub fn process_order(&self, order: Order) -> Result<Vec<Trade>, MatchingError> {
        let mut book = self.inner.lock().unwrap();
        let result = book.match_order(order)?;
        
        // Update stats outside the lock
        if !result.0.is_empty() {
            self.stats.trade_count.fetch_add(result.0.len() as u64, Ordering::Relaxed);
            if let Some(last_trade) = result.0.last() {
                self.stats.last_price.store(Some(last_trade.price));
            }
        }
        
        Ok(result.0)
    }
    
    /// Read-only operations that don't require full lock
    pub fn best_prices(&self) -> (Option<Decimal>, Option<Decimal>) {
        // Try a quick read from cached values first
        // Only lock if necessary
        let book = self.inner.lock().unwrap();
        (book.best_bid(), book.best_ask())
    }
}
```

### Performance-Critical Data Structure Selection

For absolute best performance in the order book, I recommend this approach:

1. **For price levels:**
   - Use `BTreeMap` for its natural ordering and reasonable performance
   - The O(log n) complexity is not a bottleneck for typical number of price levels
   - Pre-cache best bid/ask prices to avoid tree traversal for common operations

2. **For orders at each price level:**
   - Use plain `VecDeque` instead of `RwLock` or `Arc` wrappers
   - VecDeque gives O(1) operations at both ends (perfect for FIFO matching)
   - Pre-allocate with realistic capacity to avoid reallocations

3. **For order lookups:**
   - Use `HashMap` with a capacity hint for O(1) lookups
   - Keep order references minimal (just IDs and positions)

4. **Concurrency model:**
   - For MVP: Single `Mutex` around the entire order book is simple and effective
   - Atomic stats tracking outside the lock for non-blocking reads
   - This approach is simpler than fine-grained locking and performs well enough for most use cases

This approach offers the best balance of simplicity, performance, and maintainability while avoiding over-engineering.

## Scalability Considerations

### Immediate Scalability Options

1. **Multi-threading for the MVP**
   ```rust
   pub struct MatchingEngine {
       // Use Arc to share the order book safely between threads
       order_book: Arc<RwLock<OrderBook>>,
       depth_module: Arc<RwLock<DepthModule>>,
       // Channels for asynchronous event processing
       event_tx: mpsc::Sender<Event>,
       event_rx: mpsc::Receiver<Event>,
   }
   ```

2. **Asynchronous Event Processing**
   ```rust
   // In MatchingEngine
   pub fn start_event_loop(&self) -> JoinHandle<()> {
       let event_rx = self.event_rx.clone();
       let event_processor = self.event_processor.clone();
       
       tokio::spawn(async move {
           while let Some(event) = event_rx.recv().await {
               event_processor.process(event).await;
           }
       })
   }
   ```

3. **Lock-Free Data Structures (where appropriate)**
   ```rust
   // For frequently accessed statistics
   pub struct OrderBookStats {
       num_orders: AtomicU64,
       num_trades: AtomicU64,
       // ... other stats
   }
   ```

### Future Scalability Path

1. **Multiple Instruments Support**
   ```rust
   pub struct MultiInstrumentEngine {
       // Map of instrument IDs to their dedicated MatchingEngines
       engines: HashMap<Uuid, Arc<MatchingEngine>>,
       // Router for incoming orders
       order_router: OrderRouter,
   }
   ```

2. **Sharding by Instrument**
   ```rust
   pub struct ShardManager {
       // Sharding strategy (e.g., by instrument ID hash)
       shard_strategy: Box<dyn ShardStrategy>,
       // Shard instances
       shards: Vec<Arc<InstrumentShard>>,
   }
   ```

3. **Persistence and Recovery**
   ```rust
   pub struct PersistenceManager {
       // Event log for durability
       event_log: EventLog,
       // Snapshot manager for faster recovery
       snapshot_manager: SnapshotManager,
   }
   ```

## Technology Stack for Robustness and Performance

To implement this architecture in a robust and scalable way, we'll leverage the following Rust technologies:

### Data Structures

1. **BTreeMap** for the order book price levels
   - O(log n) lookup complexity
   - Naturally sorted, ideal for finding best prices
   - Memory efficient compared to alternatives

2. **VecDeque** for order queues within price levels
   - O(1) push and pop from both ends
   - Efficient for time-priority ordering

3. **HashMap** for O(1) lookups of orders by ID
   - Fast cancellation and modification operations

### Concurrency & Performance

1. **tokio** for async runtime
   - High-performance event-driven architecture
   - Multi-threaded task scheduling
   - Built-in sync primitives like channels and mutexes

2. **Arc (Atomic Reference Counting)** for shared ownership
   - Thread-safe shared references
   - Efficient for read-heavy workloads

3. **RwLock** for concurrent access to shared resources
   - Prioritizes readers for depth information
   - Efficient for read-heavy workloads (depth queries)

4. **Channels** for asynchronous communication
   - `tokio::sync::mpsc` for high-throughput messaging
   - Backpressure handling for stability under load

### Performance Optimization

1. **Arena Allocators (bumpalo)** for memory management
   - Reduced allocation overhead in hot paths
   - Bulk deallocations for better performance

2. **Atomic Operations** for lockless counters and statistics
   - Reduced contention for frequently updated values
   - Better scaling across multiple cores

### Robustness

1. **thiserror** for ergonomic error handling
   - Well-defined error types
   - Type-safe error propagation

2. **tracing** for structured logging and diagnostics
   - Contextual logging for easier debugging
   - Sampling and filtering capabilities

3. **metrics** for performance monitoring
   - Real-time performance insights
   - Early detection of performance issues

## Conclusion

This MVP architecture provides a solid foundation for a matching engine with the minimum necessary components. By focusing on just two order types (GTC and IOC) and treating market orders directly as IOCs, we create a simple, robust system that can be extended as requirements grow.

The key aspects of this simplified design are:

1. **Focus on Core Components**: OrderBook, MatchingEngine, and DepthModule provide the minimal functionality needed.

2. **Streamlined Order Types**: Just two fundamental behaviors (GTC and IOC) to simplify logic.

3. **Built for Scalability**: Even at the MVP stage, we design with future scaling in mind.

4. **Leveraging Rust's Strengths**: Using Rust's powerful type system, memory safety, and performance characteristics.

This architecture provides a clear path from MVP to a fully-featured, high-performance matching engine. 
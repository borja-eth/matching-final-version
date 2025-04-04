# Matching Engine Architecture

## Overview

This document outlines the architecture of a high-performance matching engine implemented in Rust. The design prioritizes performance, reliability, and maintainability while following Rust best practices.

## System Flow

```
                                 HIGH-LEVEL FLOW
┌────────────────┐      ┌────────────────┐      ┌────────────────┐      ┌────────────────┐
│                │      │                │      │                │      │                │
│  API Gateway   │──────▶  Order Router  │──────▶ Matching Core  │──────▶  Event Stream  │
│                │      │                │      │                │      │                │
└────────────────┘      └────────────────┘      └────────────────┘      └────────────────┘
        │                       │                       │                       │
        │                       │                       │                       │
        ▼                       ▼                       ▼                       ▼
┌────────────────┐      ┌────────────────┐      ┌────────────────┐      ┌────────────────┐
│   Validation   │      │  Instrument    │      │  Order Books   │      │   Persistence  │
│     Layer      │      │    Registry    │      │                │      │     Layer      │
└────────────────┘      └────────────────┘      └────────────────┘      └────────────────┘
```

### Detailed Order Flow

```
                            DETAILED ORDER FLOW
┌──────────────────────────────────────────────────────────────────────────────────────┐
│                                                                                      │
│  ┌──────────┐     ┌───────────┐    ┌──────────────┐    ┌────────────┐               │
│  │          │     │           │    │              │    │            │               │
│  │  Order   │────▶│ Validation│───▶│ Instrument   │───▶│ Matching   │               │
│  │  Intake  │     │           │    │ Dispatcher   │    │ Engine     │               │
│  │          │     │           │    │              │    │            │               │
│  └──────────┘     └───────────┘    └──────────────┘    └────────────┘               │
│       │                │                  │                  │                       │
│       │                │                  │                  │                       │
│       ▼                ▼                  ▼                  ▼                       │
│  ┌──────────┐     ┌───────────┐    ┌──────────────┐    ┌────────────┐     ┌─────────┐
│  │          │     │           │    │              │    │            │     │         │
│  │  Client  │     │  Error    │    │  Instrument  │    │  Trade     │────▶│ Event   │
│  │ Response │     │  Handling │    │  Shards      │    │ Generation │     │ Stream  │
│  │          │     │           │    │              │    │            │     │         │
│  └──────────┘     └───────────┘    └──────────────┘    └────────────┘     └─────────┘
│                                           │                  │                 │     │
│                                           │                  │                 │     │
│                                           ▼                  ▼                 ▼     │
│                                    ┌──────────────┐    ┌────────────┐    ┌──────────┐
│                                    │              │    │            │    │          │
│                                    │  Order Books │    │ Settlement │    │ Storage  │
│                                    │  (Per Instr.)│    │ Processing │    │          │
│                                    │              │    │            │    │          │
│                                    └──────────────┘    └────────────┘    └──────────┘
│                                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Gateway Layer

* **HTTP/WebSocket API**: External interface for clients to submit orders, cancel requests, and receive market data.
* **FIX Protocol Adapter**: (Optional) For institutional clients.
* **Connection Management**: Handles authentication, rate limiting, and session management.

### 2. Order Validation

* **Schema Validation**: Ensures all required fields are present and correctly typed.
* **Business Rule Validation**: Verifies order constraints (size, price, etc.) before acceptance.
* **Risk Checks**: Basic pre-trade risk checks. More complex risk management is handled separately.

### 3. Order Router

* **Instrument Registry**: Contains metadata about all tradable instruments.
* **Shard Selection**: Routes orders to the appropriate instrument shard.
* **Load Balancing**: Distributes orders evenly when multiple engines handle the same instrument.

### 4. Matching Core

#### Instrument Shard Manager

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│                     INSTRUMENT SHARD MANAGER                     │
│                                                                  │
│  ┌────────────┐   ┌────────────┐   ┌────────────┐               │
│  │            │   │            │   │            │               │
│  │ Instrument │   │ Instrument │   │ Instrument │  ...          │
│  │  Shard #1  │   │  Shard #2  │   │  Shard #3  │               │
│  │            │   │            │   │            │               │
│  └────────────┘   └────────────┘   └────────────┘               │
│        │                │                │                       │
│        ▼                ▼                ▼                       │
│  ┌────────────┐   ┌────────────┐   ┌────────────┐               │
│  │            │   │            │   │            │               │
│  │ Order Book │   │ Order Book │   │ Order Book │  ...          │
│  │    BTC     │   │    ETH     │   │    SOL     │               │
│  │            │   │            │   │            │               │
│  └────────────┘   └────────────┘   └────────────┘               │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

* Each instrument has a dedicated matching engine shard (logical or physical).
* Shards operate independently to maximize throughput.
* Shards can scale horizontally for high-volume instruments.

#### Order Book Structure (Per Instrument)

```
┌────────────────────────────────────────────────────┐
│                                                    │
│                    ORDER BOOK                      │
│                                                    │
│  ┌────────────────┐        ┌────────────────┐     │
│  │                │        │                │     │
│  │    Buy Side    │        │   Sell Side    │     │
│  │                │        │                │     │
│  │  ┌──────────┐  │        │  ┌──────────┐  │     │
│  │  │Price Lvl1│  │        │  │Price Lvl1│  │     │
│  │  │ 50,000   │  │        │  │ 50,100   │  │     │
│  │  └──────────┘  │        │  └──────────┘  │     │
│  │      │         │        │      │         │     │
│  │      ▼         │        │      ▼         │     │
│  │  ┌──────────┐  │        │  ┌──────────┐  │     │
│  │  │Price Lvl2│  │        │  │Price Lvl2│  │     │
│  │  │ 49,950   │  │        │  │ 50,200   │  │     │
│  │  └──────────┘  │        │  └──────────┘  │     │
│  │      │         │        │      │         │     │
│  │      ▼         │        │      ▼         │     │
│  │  ┌──────────┐  │        │  ┌──────────┐  │     │
│  │  │Price Lvl3│  │        │  │Price Lvl3│  │     │
│  │  │ 49,900   │  │        │  │ 50,300   │  │     │
│  │  └──────────┘  │        │  └──────────┘  │     │
│  │      │         │        │      │         │     │
│  │      ▼         │        │      ▼         │     │
│  │     ...        │        │     ...        │     │
│  │                │        │                │     │
│  └────────────────┘        └────────────────┘     │
│                                                    │
│  ┌────────────────────────────────────────────┐   │
│  │                                            │   │
│  │           Trigger Order Pool               │   │
│  │   (Stop/StopLimit orders waiting trigger)  │   │
│  │                                            │   │
│  └────────────────────────────────────────────┘   │
│                                                    │
└────────────────────────────────────────────────────┘
```

* Price level organization with time priority within levels
* Separate structures for buy and sell sides
* Special handling for trigger-based orders (Stop, StopLimit)

### 5. Event Stream

* **Trade Events**: Generated when orders match.
* **Order Status Events**: Track lifecycle changes of orders.
* **Market Data Events**: Book updates, price changes, etc.
* **Event Bus**: Central channel for all system events.

### 6. Persistence Layer

* **Event Log**: Append-only log of all actions for recovery/audit.
* **State Snapshots**: Periodic snapshots of order books for faster recovery.
* **Trade History**: Historical record of all executed trades.

## Code Style and Documentation Standards

### File Structure and Organization

Every file should begin with a module overview using the following template:

```rust
//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements [brief description of module purpose]
//
// | Section             | Description                                                   |
// |---------------------|---------------------------------------------------------------|
// | CONSTANTS           | Configuration and system constants                            |
// | TYPES               | Core data structures and type definitions                     |
// | TRAITS              | Trait definitions for the module's abstractions               |
// | IMPLEMENTATIONS     | Implementations of the core data structures                   |
// | FUNCTIONS           | Public and private utility functions                          |
// | ERROR HANDLING      | Error types and handling logic                                |
// | TESTS               | Unit and integration tests                                    |
//--------------------------------------------------------------------------------------------------
```

### Documentation Requirements

1. **Module Documentation**
   * Every module must have a `//!` doc comment explaining its purpose
   * Include any necessary usage examples

   ```rust
   //! # Order Book Module
   //! 
   //! This module implements a price-time priority order book for matching limit orders.
   //! It supports:
   //! 
   //! - Fast insertion and retrieval of orders
   //! - Time-priority ordering within price levels
   //! - Efficient matching algorithm for market and limit orders
   //! 
   //! ## Usage Example
   //! 
   //! ```
   //! let mut book = OrderBook::new(instrument_id);
   //! book.add_order(order)?;
   //! let trades = book.match_order(incoming_order)?;
   //! ```
   ```

2. **Public Item Documentation**
   * All public items (`pub`) must have comprehensive `///` doc comments
   * Include:
     * Purpose/function of the item
     * Parameter descriptions (for functions)
     * Return value descriptions
     * Error conditions and types
     * Usage examples for complex items

   ```rust
   /// Attempts to match an incoming order against the order book.
   ///
   /// This function implements the core matching logic for the engine. For market orders,
   /// it will walk through the opposite side of the book until the order is filled or
   /// the book is exhausted. For limit orders, it will match against eligible opposite
   /// orders that meet the price requirements.
   ///
   /// # Arguments
   ///
   /// * `order` - The incoming order to match. This will be modified to reflect filled quantities.
   ///
   /// # Returns
   ///
   /// A vector of trades generated from the matching process. Empty if no matches occurred.
   ///
   /// # Errors
   ///
   /// Returns `MatchingError::InvalidOrderType` if the order type is not supported for matching.
   /// Returns `MatchingError::InsufficientLiquidity` if a market order cannot be completely filled.
   ///
   /// # Examples
   ///
   /// ```
   /// let mut book = OrderBook::new(instrument_id);
   /// // Add some orders to the book
   /// book.add_order(sell_order_1)?;
   /// book.add_order(sell_order_2)?;
   ///
   /// // Match a buy order against the book
   /// let mut buy_order = Order::new(/* ... */);
   /// let trades = book.match_order(&mut buy_order)?;
   /// ```
   pub fn match_order(&mut self, order: &mut Order) -> Result<Vec<Trade>, MatchingError> {
       // Implementation
   }
   ```

3. **Component API Documentation**
   * Each major component should have a dedicated `API.md` file
   * Document public interfaces, expected behavior, and integration points

4. **Performance-Critical Sections**
   * For performance-critical code, include benchmarking results in comments
   * Document any optimizations and their rationale

   ```rust
   /// Finds the best matching price level for the given order.
   /// 
   /// # Performance Notes
   /// 
   /// Uses a B-tree map with O(log n) lookup complexity.
   /// Benchmarks show ~100ns average lookup time with 10,000 price levels.
   /// 
   /// Critical path optimization: we use a hint-based lookup when possible
   /// to reduce traversal time.
   fn find_best_match(&self, price: Decimal, side: Side) -> Option<&PriceLevel> {
       // Implementation
   }
   ```

### Code Organization Example

```rust
//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements the order book for a single instrument.
//
// | Section             | Description                                                   |
// |---------------------|---------------------------------------------------------------|
// | TYPES               | Order book data structures                                    |
// | IMPLEMENTATIONS     | OrderBook implementation                                      |
// | HELPER FUNCTIONS    | Internal utility functions                                    |
// | ERROR HANDLING      | OrderBook-specific errors                                     |
// | TESTS               | Unit tests for order book functionality                       |
//--------------------------------------------------------------------------------------------------

use crate::types::{Order, OrderStatus, Side, Trade};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap, VecDeque};
use thiserror::Error;
use uuid::Uuid;

//--------------------------------------------------------------------------------------------------
// TYPES
//--------------------------------------------------------------------------------------------------

/// Represents a price level in the order book, containing all orders at that price.
pub struct PriceLevel {
    /// The price of this level
    pub price: Decimal,
    /// Orders at this price level in time priority (oldest first)
    orders: VecDeque<Order>,
    /// Total volume available at this level
    total_volume: Decimal,
}

/// The core order book structure for a single instrument.
pub struct OrderBook {
    /// Instrument ID this order book belongs to
    instrument_id: Uuid,
    /// Buy side of the book, ordered by price descending (highest first)
    buy_levels: BTreeMap<Decimal, PriceLevel>,
    /// Sell side of the book, ordered by price ascending (lowest first)
    sell_levels: BTreeMap<Decimal, PriceLevel>,
    /// Map of order IDs to their location in the book for fast lookups
    order_map: HashMap<Uuid, OrderLocation>,
}

/// Tracks the location of an order in the book for fast retrieval.
struct OrderLocation {
    side: Side,
    price: Decimal,
    position: usize,
}

//--------------------------------------------------------------------------------------------------
// ERROR HANDLING
//--------------------------------------------------------------------------------------------------

/// Errors that can occur during order book operations.
#[derive(Error, Debug)]
pub enum OrderBookError {
    /// The requested order was not found in the book
    #[error("Order {0} not found in the book")]
    OrderNotFound(Uuid),
    
    /// Attempted to add an order that already exists
    #[error("Order {0} already exists in the book")]
    DuplicateOrder(Uuid),
    
    /// Invalid order configuration
    #[error("Invalid order: {0}")]
    InvalidOrder(String),
}

//--------------------------------------------------------------------------------------------------
// IMPLEMENTATIONS
//--------------------------------------------------------------------------------------------------

impl PriceLevel {
    /// Creates a new price level at the specified price.
    pub fn new(price: Decimal) -> Self {
        Self {
            price,
            orders: VecDeque::new(),
            total_volume: Decimal::ZERO,
        }
    }
    
    /// Adds an order to this price level.
    pub fn add_order(&mut self, order: Order) {
        self.total_volume += order.remaining_base;
        self.orders.push_back(order);
    }
    
    // Additional methods...
}

impl OrderBook {
    /// Creates a new, empty order book for the specified instrument.
    pub fn new(instrument_id: Uuid) -> Self {
        Self {
            instrument_id,
            buy_levels: BTreeMap::new(),
            sell_levels: BTreeMap::new(),
            order_map: HashMap::new(),
        }
    }
    
    /// Adds an order to the book.
    pub fn add_order(&mut self, order: Order) -> Result<(), OrderBookError> {
        // Implementation...
        Ok(())
    }
    
    /// Retrieves the best bid price level, if any.
    pub fn best_bid(&self) -> Option<&PriceLevel> {
        self.buy_levels.iter().next().map(|(_, level)| level)
    }
    
    /// Retrieves the best ask price level, if any.
    pub fn best_ask(&self) -> Option<&PriceLevel> {
        self.sell_levels.iter().next().map(|(_, level)| level)
    }
    
    // Additional methods...
}

//--------------------------------------------------------------------------------------------------
// TESTS
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_order() {
        // Test implementation
    }
    
    #[test]
    fn test_match_orders() {
        // Test implementation
    }
}
```

## Rust-Specific Design Patterns

### 1. Type System Utilization

* **Domain-Driven Design with Types**
   ```rust
   // Use newtypes to make intent clear and prevent errors
   pub struct OrderId(pub Uuid);
   pub struct Price(pub Decimal);
   pub struct Quantity(pub Decimal);

   // Make illegal states unrepresentable
   pub enum OrderState {
       New { id: OrderId, created_at: DateTime<Utc> },
       Active { id: OrderId, placed_at: DateTime<Utc> },
       Filled { id: OrderId, filled_at: DateTime<Utc> },
       Cancelled { id: OrderId, reason: CancelReason },
   }
   ```

* **Error Handling with `thiserror`**
   ```rust
   #[derive(Debug, Error)]
   pub enum MatchingError {
       #[error("Order {0} not found")]
       OrderNotFound(OrderId),
       
       #[error("Insufficient quantity: required {required}, available {available}")]
       InsufficientQuantity {
           required: Quantity,
           available: Quantity,
       },
       
       #[error("Invalid price: {0}")]
       InvalidPrice(Price),
       
       #[error("Internal error: {0}")]
       Internal(String),
   }
   ```

### 2. Ownership and Borrowing Patterns

* **Command Pattern with Ownership Transfer**
   ```rust
   pub enum OrderCommand {
       Place(Order),              // Takes ownership of the order
       Cancel { id: OrderId },    // Only needs the ID
       Modify { id: OrderId, new_price: Option<Price>, new_quantity: Option<Quantity> },
   }
   ```

* **Immutable Borrow for Queries**
   ```rust
   impl OrderBook {
       // Immutable borrow for read operations
       pub fn best_bid(&self) -> Option<&PriceLevel> {
           self.buy_levels.values().next()
       }
       
       // Mutable borrow for modifications
       pub fn add_order(&mut self, order: Order) -> Result<(), MatchingError> {
           // Implementation
       }
   }
   ```

### 3. Trait-Based Abstractions

* **Behavior Traits**
   ```rust
   pub trait OrderMatcher {
       fn match_order(&mut self, order: &mut Order) -> Result<Vec<Trade>, MatchingError>;
   }
   
   pub trait OrderBook {
       fn add_order(&mut self, order: Order) -> Result<(), MatchingError>;
       fn cancel_order(&mut self, id: OrderId) -> Result<Order, MatchingError>;
       fn best_price(&self, side: Side) -> Option<Price>;
   }
   ```

* **Trait for Event Handling**
   ```rust
   pub trait EventHandler {
       fn handle_trade_event(&mut self, event: TradeEvent);
       fn handle_order_event(&mut self, event: OrderEvent);
   }
   ```

### 4. Interior Mutability Pattern

* **For Shared State Without Mutexes**
   ```rust
   pub struct Statistics {
       trade_count: AtomicU64,
       total_volume: Decimal, // Protected by RwLock internally when needed
   }
   
   impl Statistics {
       pub fn increment_trade_count(&self) {
           self.trade_count.fetch_add(1, Ordering::SeqCst);
       }
   }
   ```

### 5. Builder Pattern

* **For Complex Object Construction**
   ```rust
   pub struct OrderBuilder {
       id: Option<OrderId>,
       side: Option<Side>,
       order_type: Option<OrderType>,
       price: Option<Price>,
       quantity: Option<Quantity>,
       // Other fields
   }
   
   impl OrderBuilder {
       pub fn new() -> Self {
           Self {
               id: None,
               side: None,
               order_type: None,
               price: None,
               quantity: None,
           }
       }
       
       pub fn id(mut self, id: OrderId) -> Self {
           self.id = Some(id);
           self
       }
       
       pub fn side(mut self, side: Side) -> Self {
           self.side = Some(side);
           self
       }
       
       // Other builder methods
       
       pub fn build(self) -> Result<Order, ValidationError> {
           // Validate and construct the Order
       }
   }
   ```

### 6. State Machine Pattern

* **For Order Lifecycle Management**
   ```rust
   impl Order {
       pub fn try_fill(&mut self, quantity: Quantity, price: Price) -> Result<OrderStatus, MatchingError> {
           match self.status {
               OrderStatus::New | OrderStatus::PartiallyFilled => {
                   if quantity > self.remaining_quantity {
                       return Err(MatchingError::InsufficientQuantity {
                           required: quantity,
                           available: self.remaining_quantity,
                       });
                   }
                   
                   self.remaining_quantity -= quantity;
                   self.filled_quantity += quantity;
                   
                   if self.remaining_quantity.is_zero() {
                       self.status = OrderStatus::Filled;
                   } else {
                       self.status = OrderStatus::PartiallyFilled;
                   }
                   
                   Ok(self.status)
               },
               _ => Err(MatchingError::InvalidState(format!(
                   "Cannot fill order in state {:?}", self.status
               ))),
           }
       }
   }
   ```

## Implementation Strategy

### Instrument Shard Manager

```rust
pub struct InstrumentShardManager {
    /// Maps instrument IDs to their dedicated shard
    instrument_map: HashMap<Uuid, InstrumentShardHandle>,
    /// Channel to communicate with shards
    command_tx: mpsc::Sender<ShardCommand>,
}
```

### Per-Instrument Matching Engine

```rust
pub struct MatchingEngineShard {
    /// Instrument this shard is responsible for
    instrument_id: Uuid,
    /// The limit order book for this instrument
    order_book: OrderBook,
    /// Pool of trigger orders waiting to activate
    trigger_order_pool: TriggerOrderPool,
    /// Command receiver from the shard manager
    command_rx: mpsc::Receiver<ShardCommand>,
    /// Trade event sender
    trade_event_tx: mpsc::Sender<TradeEvent>,
}
```

### Order Book Implementation

```rust
pub struct OrderBook {
    /// Buy side of the book (price descending)
    buy_levels: BTreeMap<Decimal, PriceLevel>,
    /// Sell side of the book (price ascending)
    sell_levels: BTreeMap<Decimal, PriceLevel>,
    /// Map of active orders for quick lookups
    orders: HashMap<Uuid, OrderRef>,
}

pub struct PriceLevel {
    /// Price of this level
    price: Decimal,
    /// Orders at this price level in time priority
    orders: VecDeque<Order>,
    /// Total volume available at this level
    volume: Decimal,
}
```

## Data Flow Sequence

1. **Order Submission**
   * Client submits order via API/Gateway
   * Order validated and routed to correct instrument shard

2. **Order Processing**
   * For limit orders: Added to the order book at the appropriate price level
   * For market orders: Matched immediately against the opposite side
   * For trigger orders: Added to trigger pool and monitored

3. **Matching Process**
   * When a new order arrives, matching engine attempts to match it
   * Matches generate trades and update order statuses
   * Remaining quantity (if any) placed on the book (for limit orders)

4. **Event Generation**
   * All state changes generate events (new order, match, fill, cancel)
   * Events published to event bus for consumption by other services

5. **Client Notification**
   * Order status updates sent back to client
   * Market data updates broadcast to subscribers

## Recovery and Fault Tolerance

1. **Event Sourcing**
   * All state changes logged as events
   * System can be recovered by replaying events

2. **Checkpointing**
   * Periodic snapshots of order book state
   * Faster recovery by loading latest snapshot and replaying only recent events

3. **Consistency Checks**
   * Regular verification of order book integrity
   * Automated reconciliation processes

## Future Extensions

1. **Advanced Order Types**
   * Iceberg/Reserve orders
   * Time-in-force variations (IOC, FOK, GTD)

2. **Market Protections**
   * Circuit breakers
   * Price banding
   * Anti-manipulation controls

3. **Performance Enhancements**
   * FPGA acceleration for critical paths
   * Custom memory management
   * Further optimization based on profiling results

## Architectural Decisions

### Multi-Instrument Support

We'll use a shard-per-instrument approach:

1. **Instrument Registry**
   * Central registry containing all instrument definitions
   * Maps instrument IDs to specific shards

2. **Shard Management**
   * Each instrument gets a dedicated matching engine instance
   * For very high-volume instruments, consider multiple shards with consistent hashing

3. **Cross-Instrument Operations**
   * Special handling for operations involving multiple instruments
   * Coordination via the event bus

### Performance Considerations

1. **Memory Management**
   * Pre-allocate memory for order books and critical data structures
   * Use arena allocators (`bumpalo`) for short-lived allocations within a matching cycle
   * Minimize heap allocations in hot paths

2. **Concurrency Model**
   * Each instrument shard runs in its own thread
   * Message-passing between components using channels (`tokio::sync::mpsc`, `crossbeam-channel`)
   * Minimize shared state and locks

3. **Data Structure Selection**
   * Price levels: `BTreeMap` for ordered access
   * Order storage: Custom data structures with pre-allocation
   * Consider custom lock-free structures only if benchmarks justify complexity

4. **Latency Optimization**
   * Critical path analysis and profiling-driven optimization
   * Batching of non-critical operations
   * Careful error handling that doesn't block the matching loop 
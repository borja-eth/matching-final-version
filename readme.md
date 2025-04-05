1- Every order type and TimeInForce has its own specialized hot path for better 
performance.
2- Check that there are no placeholders across files.
3- We must run everything on a single thread

# Ultimate-Matching: High-Performance Trading Matching Engine

A high-performance, event-driven matching engine for trading systems implemented in Rust. Designed for low-latency, high-throughput environments with price-time priority matching.

```
                                      ┌───────────────┐
                                      │   Orders      │
                                      │      ↓ 1      │
                                      └───────┬───────┘
                                              │
┌───────────────────────────────────────────────────────────────────────┐
│                          Matching Engine                               │
│  ┌─────────────┐        ┌─────────────┐        ┌─────────────────┐    │
│  │             │ 4a/5   │      2      │   6    │                 │    │
│  │  OrderBook  ◄────────►  Matcher    ├────────►  DepthTracker   │    │
│  │             │        │             │        │                 │    │
│  └──────┬──────┘        └──────┬──────┘        └─────────────────┘    │
│         │                      │                                       │
│  ┌──────▼──────┐        ┌──────▼──────┐                               │
│  │     4b      │        │      3      │                               │
│  │ PriceLevel  │        │  MatchResult│                               │
│  │             │        │             │                               │
│  └─────────────┘        └──────┬──────┘                               │
│                                │                                       │
└────────────────────────────────┼───────────────────────────────────────┘
                                 │ 7
                         ┌───────▼──────┐
                         │              │
                         │  Event Bus   │
                         │              │
                         └──────┬───────┘
                                │ 8
                 ┌──────────────┼──────────────┐
                 │              │              │
         ┌───────▼──────┐┌──────▼──────┐┌──────▼──────┐
         │      9a      ││     9b      ││     9c      │
         │EventDispatcher││ EventLogger  ││Persistence   │
         │              ││              ││ Handler      │
         └──────────────┘└──────────────┘└──────────────┘
```

## Order Flow Steps

1. **Order Entry**: Orders enter the matching engine system
2. **Matcher Processing**: The matcher processes the order based on type (Limit, Market, Stop, StopLimit) and time-in-force (GTC, IOC)
3. **Match Result Creation**: If matched, trades are generated and a MatchResult is created
4. **Order Book Update**:
   - a. Resting orders are stored in the OrderBook if not fully filled (for GTC orders)
   - b. Orders are grouped by price in PriceLevels
5. **Order Retrieval**: When matching, orders are retrieved from the OrderBook based on price-time priority
6. **Depth Update**: DepthTracker is updated with changes to the OrderBook
7. **Event Emission**: Events (trades, order updates) are emitted via EventBus
8. **Event Distribution**: EventDispatcher routes events to registered handlers
9. **Event Processing**:
   - a. EventDispatcher delivers events to appropriate handlers
   - b. EventLogger maintains an in-memory log of events
   - c. PersistenceHandler writes events to disk for durability

## Event System Details

### Event Types

The matching engine emits the following event types during order processing:

| Event Type | Description | Emitted When |
|------------|-------------|--------------|
| `OrderAdded` | An order was added to the order book | When a new order is added to the book (after step 4a) |
| `OrderMatched` | An order was matched with another order | When an order is partially or fully matched (step 3) |
| `OrderCancelled` | An order was cancelled | When cancel_order() is called or IOC orders expire |
| `OrderStatusChanged` | An order's status was updated | When order status transitions between states |
| `TradeExecuted` | A trade was executed between two orders | When a match generates a trade (step 3) |
| `DepthUpdated` | The order book depth has changed | After order book updates (step 6) |

### Event Flow

1. **Emitter**: The `MatchingEngine` emits events
   - After processing orders (OrderAdded, OrderMatched)
   - After generating trades (TradeExecuted)
   - After updating order statuses (OrderStatusChanged, OrderCancelled)
   - After updating depth (DepthUpdated)

2. **Transport**: Events flow through the `EventBus`
   - Acts as a central message broker
   - Offers a publish-subscribe pattern
   - Non-blocking (uses Tokio broadcast channels)
   - Events are cloned when published

3. **Receiver**: Events are received by handlers implementing the `EventHandler` trait
   - `EventDispatcher`: Routes events to appropriate handlers
   - `EventLogger`: Logs events in memory for debugging/audit
   - `PersistenceEventHandler`: Writes events to storage (JSONL files)
   - Custom handlers: Can be implemented by users for specific needs

### Event Timing

- **Order Added**: Emitted after an order is added to the order book (non-matched portion)
- **Order Matched**: Emitted for both the new (taker) order and the matched (maker) orders
- **Trade Executed**: Emitted after a match is confirmed and a trade is generated
- **Order Cancelled**: Emitted when an order is explicitly cancelled or expires (IOC)
- **Depth Updated**: Emitted when order book depth is queried after changes

### Event Flow Example

For a typical limit order that gets partially matched:

1. Order arrives at the matching engine
2. Engine attempts to match it with existing orders
3. If partially filled:
   - `TradeExecuted` event for each matching trade
   - `OrderMatched` event for the incoming order (partial fill)
   - `OrderMatched` event for each matched resting order
4. Remaining quantity added to order book
   - `OrderAdded` event for the remaining quantity
5. Depth is updated
   - `DepthUpdated` event with the new state

## Core Components

| Component Type | Name | Description |
|----------------|------|-------------|
| **Structs** | | |
| | `OrderBook` | Maintains bids and asks in price-time priority |
| | `PriceLevel` | Groups orders at the same price |
| | `MatchingEngine` | Core matching logic for processing orders |
| | `MatchResult` | Outcome of a matching operation |
| | `Order` | Trading order with all attributes |
| | `Trade` | Executed trade between two orders |
| | `DepthTracker` | Maintains real-time aggregated order book views |
| | `DepthSnapshot` | Point-in-time view of order book depth |
| | `EventBus` | Central hub for publishing and subscribing to events |
| | `EventDispatcher` | Routes events to registered handlers |
| | `EventLogger` | Simple in-memory event logger |
| | `PersistenceEventHandler` | Writes events to disk |
| **Traits** | | |
| | `EventHandler` | Interface for components that process events |
| **Enums** | | |
| | `Side` | Order side (Bid/Ask) |
| | `OrderType` | Type of order (Limit, Market, Stop, StopLimit) |
| | `OrderStatus` | Lifecycle status of an order |
| | `TimeInForce` | Duration policy for orders (GTC, IOC) |
| | `MatchingEngineEvent` | Event types in the system |
| | `MatchingError` | Errors that can occur during matching |
| **Type Aliases** | | |
| | `MatchingResult<T>` | Result with MatchingError |
| | `EventResult<T>` | Result with EventError |

## Order Book & Matching Engine Features

| Feature | Description | Implementation |
|---------|-------------|----------------|
| **Price-Time Priority** | Orders are matched in price-time priority (FIFO) | BTreeMap + VecDeque |
| **Order Types** | Limit, Market, Stop, Stop-Limit | OrderType enum + specialized processing paths |
| **Time-In-Force** | GTC (Good Till Cancel), IOC (Immediate Or Cancel) | TimeInForce enum + specialized processing |
| **Order Management** | Add, cancel, and match orders | add_order, cancel_order, match_order functions |
| **Order Book Queries** | Best bid/ask, spread, depth, volume | best_bid/ask, spread, get_snapshot functions |
| **Depth Tracking** | Real-time aggregated order book views | DepthTracker + DepthSnapshot |
| **Event System** | Non-blocking event processing | EventBus + EventDispatcher + EventHandler trait |
| **Persistence** | Event logging and storage | PersistenceEventHandler |
| **Thread Safety** | Shared components across threads | Arc + RwLock + Mutex |
| **Performance** | Optimized for low latency | Inlining, minimal allocation, batch processing |
| **Error Handling** | Comprehensive error types | Result + Error enums |
| **Price Representation** | Fixed-point decimal pricing | rust_decimal::Decimal |
| **Concurrency** | Async event processing | Tokio runtime |

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/ultimate-matching.git
cd ultimate-matching

# Build the project
cargo build --release

# Run the tests
cargo test
```

## Usage

```rust
use ultimate_matching::{
    matching_engine::MatchingEngine,
    types::{Order, Side, OrderType, TimeInForce},
};
use uuid::Uuid;

// Create a new matching engine for an instrument
let instrument_id = Uuid::new_v4();
let mut engine = MatchingEngine::new(instrument_id);

// Create and process an order
let order = Order::new(
    Uuid::new_v4(),
    Side::Bid,
    OrderType::Limit,
    dec!(100.0),
    dec!(1.0),
    instrument_id,
);

// Process the order with GTC time in force
match engine.process_order(order, TimeInForce::GTC) {
    Ok(result) => {
        // Handle the match result (trades, affected orders, etc.)
        println!("Order processed: {:?}", result);
    },
    Err(e) => {
        // Handle error
        println!("Error processing order: {:?}", e);
    }
}

// Get a depth snapshot
let depth = engine.get_depth(10);
println!("Order book depth: {:?}", depth);
```

## Documentation

### OrderBook Module

The `OrderBook` maintains bid and ask orders in price-time priority. It uses `BTreeMap` for efficient price level management and `VecDeque` for FIFO ordering within each price level.

Key methods:
- `add_order(order: Order)`: Adds an order to the book
- `remove_order(order_id: Uuid, side: Side, price: Decimal)`: Removes an order
- `peek_best_order(side: Side)`: Gets the best order without removing it
- `best_bid()/best_ask()`: Gets the best bid/ask price
- `spread()`: Calculates the current spread
- `volume_at_price(side: Side, price: Decimal)`: Gets volume at a price level

### MatchingEngine Module

The `MatchingEngine` is the core component for processing orders and generating trades. It handles different order types and time-in-force policies with specialized processing paths.

Key methods:
- `process_order(order: Order, time_in_force: TimeInForce)`: Processes an order
- `cancel_order(order_id: Uuid)`: Cancels an existing order
- `get_depth(limit: usize)`: Gets a snapshot of the current order book depth

### Depth Module

The `DepthTracker` maintains real-time aggregated views of the order book for efficient access to market data.

Key methods:
- `update_order_added(order: &Order)`: Updates depth when an order is added
- `update_order_removed(order: &Order)`: Updates depth when an order is removed
- `get_snapshot(limit: usize)`: Creates a snapshot of current depth

### Events Module

The event system enables non-blocking event processing with a publish-subscribe pattern.

Key components:
- `EventBus`: Central hub for publishing and subscribing to events
- `EventDispatcher`: Routes events to registered handlers
- `EventHandler`: Trait for components that process events
- `MatchingEngineEvent`: Enum representing all possible events in the system

## Future Improvements

1. **Performance Optimizations**:
   - Implement lock-free data structures for critical paths
   - Optimize memory allocation patterns further
   - Profile and optimize for specific hardware architectures

2. **Additional Features**:
   - FOK (Fill Or Kill) time-in-force
   - Post-only order flag
   - Self-trade prevention
   - Support for advanced order types (trailing stop, OCO)
   - Circuit breakers and market protection mechanisms

3. **Scaling**:
   - Sharding for multi-instrument support
   - Distributed matching across multiple nodes
   - Clustered deployment with leader/follower model

4. **Operational Improvements**:
   - Enhanced monitoring and metrics
   - Admin API for order book management
   - Snapshot and replay for state recovery
   - Comprehensive benchmarking suite

5. **Integration Options**:
   - FIX protocol adapter
   - WebSocket API for real-time data
   - REST API for order management
   - Integration with common risk management systems
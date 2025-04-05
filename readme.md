1- Every order type and TimeInForce has its own specialized hot path for better performance.
2- Check that there are no placeholders across files.
3- We must run everything on a single thread


# Ultimate Matching Engine

A high-performance, Rust-based matching engine with a RESTful API.

## Features

- Fast limit order book with price-time priority
- Support for multiple order types (Limit, Market, Stop, StopLimit)

- Event-driven architecture with persistence
- RESTful API using Axum
- Decimal precision for financial calculations

## API Documentation

### Base URL

```
http://localhost:3000
```

### Endpoints

#### Health Check

```
GET /health
```

Returns `200 OK` when the server is running.

#### Order Management

##### Create Order

```
POST /orders
```

Create and process a new trading order.

**Request Body:**

```json
{
  "ext_id": "client-order-123",
  "account_id": "00000000-0000-0000-0000-000000000000",
  "order_type": "Limit",
  "instrument_id": "00000000-0000-0000-0000-000000000000",
  "side": "Bid",
  "limit_price": "100.50",
  "trigger_price": null,
  "base_amount": "1.5",
  "time_in_force": "GTC"
}
```

**Response:**

```json
{
  "id": "00000000-0000-0000-0000-000000000000",
  "ext_id": "client-order-123",
  "account_id": "00000000-0000-0000-0000-000000000000",
  "order_type": "Limit",
  "instrument_id": "00000000-0000-0000-0000-000000000000",
  "side": "Bid",
  "limit_price": "100.50",
  "trigger_price": null,
  "base_amount": "1.5",
  "remaining_base": "1.5",
  "filled_base": "0",
  "filled_quote": "0",
  "status": "New",
  "created_at": "2023-01-01T12:00:00Z",
  "updated_at": "2023-01-01T12:00:00Z"
}
```

##### Cancel Order

```
DELETE /orders/:id?instrument_id=00000000-0000-0000-0000-000000000000
```

Cancel an existing order.

**Response:**

```json
{
  "id": "00000000-0000-0000-0000-000000000000",
  "ext_id": "client-order-123",
  "account_id": "00000000-0000-0000-0000-000000000000",
  "order_type": "Limit",
  "instrument_id": "00000000-0000-0000-0000-000000000000",
  "side": "Bid",
  "limit_price": "100.50",
  "trigger_price": null,
  "base_amount": "1.5",
  "remaining_base": "1.5",
  "filled_base": "0",
  "filled_quote": "0",
  "status": "Cancelled",
  "created_at": "2023-01-01T12:00:00Z",
  "updated_at": "2023-01-01T12:00:00Z"
}
```

##### Get Order

```
GET /orders/:id?instrument_id=00000000-0000-0000-0000-000000000000
```

Get details of an existing order.

**Response:**

```json
{
  "id": "00000000-0000-0000-0000-000000000000",
  "ext_id": "client-order-123",
  "account_id": "00000000-0000-0000-0000-000000000000",
  "order_type": "Limit",
  "instrument_id": "00000000-0000-0000-0000-000000000000",
  "side": "Bid",
  "limit_price": "100.50",
  "trigger_price": null,
  "base_amount": "1.5",
  "remaining_base": "1.5",
  "filled_base": "0",
  "filled_quote": "0",
  "status": "New",
  "created_at": "2023-01-01T12:00:00Z",
  "updated_at": "2023-01-01T12:00:00Z"
}
```

#### Market Data

##### Get Order Book

```
GET /instruments/:id/orderbook
```

Get the current order book for an instrument.

**Response:**

```json
{
  "bids": [
    {
      "price": "100.50",
      "volume": "1.5",
      "order_count": 1
    },
    {
      "price": "100.00",
      "volume": "2.5",
      "order_count": 2
    }
  ],
  "asks": [
    {
      "price": "101.00",
      "volume": "1.0",
      "order_count": 1
    },
    {
      "price": "101.50",
      "volume": "2.0",
      "order_count": 1
    }
  ],
  "timestamp": "2023-01-01T12:00:00Z",
  "instrument_id": "00000000-0000-0000-0000-000000000000"
}
```

##### Get Depth

```
GET /instruments/:id/depth?level=10
```

Get market depth for an instrument.

**Response:**

Same as order book response.

##### Get Trades

```
GET /instruments/:id/trades?limit=20
```

Get recent trades for an instrument.

**Response:**

```json
[
  {
    "id": "00000000-0000-0000-0000-000000000000",
    "instrument_id": "00000000-0000-0000-0000-000000000000",
    "maker_order_id": "00000000-0000-0000-0000-000000000000",
    "taker_order_id": "00000000-0000-0000-0000-000000000000",
    "base_amount": "1.0",
    "quote_amount": "100.50",
    "price": "100.50",
    "created_at": "2023-01-01T12:00:00Z"
  }
]
```

#### Instrument Management

##### Create Instrument

```
POST /instruments
```

Create a new trading instrument.

**Request Body:**

```json
{
  "id": "00000000-0000-0000-0000-000000000000", 
  "name": "BTC/USD",
  "base_currency": "BTC",
  "quote_currency": "USD"
}
```

**Response:**

```json
{
  "id": "00000000-0000-0000-0000-000000000000",
  "name": "BTC/USD",
  "base_currency": "BTC",
  "quote_currency": "USD"
}
```

##### List Instruments

```
GET /instruments
```

List all available instruments.

**Response:**

```json
[
  "00000000-0000-0000-0000-000000000000",
  "11111111-1111-1111-1111-111111111111"
]
```

## Running the API Server

```
cargo run --bin api_server
```

## Development

### Building the Project

```
cargo build
```

### Running Tests

```
cargo test
```

### Running Benchmarks

```
cargo bench
```

## Event System

The matching engine includes a high-performance event system that provides a way to react to changes in the order book without blocking the core matching operations.

### Key Event Types

- `OrderAdded` - Generated when an order is added to the book
- `OrderMatched` - Generated when an order is matched (partially or fully)
- `OrderCancelled` - Generated when an order is cancelled
- `OrderStatusChanged` - Generated when an order's status changes
- `TradeExecuted` - Generated when a trade is executed
- `DepthUpdated` - Generated when the order book depth changes

### Using the Event System

#### 1. Create the Event Bus

```rust
use ultimate_matching::events::EventBus;

// Create an event bus with default capacity (1024 events)
let event_bus = EventBus::default();

// Or create with custom capacity
let event_bus = EventBus::new(2048);
```

#### 2. Create Event Handlers

Implement the `EventHandler` trait to create custom handlers:

```rust
use ultimate_matching::events::{EventHandler, MatchingEngineEvent, EventResult};

struct MyHandler;

#[async_trait::async_trait]
impl EventHandler for MyHandler {
    fn event_types(&self) -> Vec<&'static str> {
        vec!["OrderAdded", "TradeExecuted"]
    }
    
    async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()> {
        // Process the event
        match event {
            MatchingEngineEvent::TradeExecuted { trade, .. } => {
                println!("Trade executed: {} @ {}", trade.base_amount, trade.price);
            },
            _ => {}
        }
        Ok(())
    }
}
```

#### 3. Set Up the Event Dispatcher

```rust
use std::sync::Arc;
use ultimate_matching::events::EventDispatcher;

// Create and start the dispatcher
let dispatcher = EventDispatcher::new(event_bus.clone());
let handler = Arc::new(MyHandler);
dispatcher.register_handler(handler).await;
let _handle = dispatcher.start().await;
```

#### 4. Create a Matching Engine with Events

```rust
use ultimate_matching::MatchingEngine;
use uuid::Uuid;

let instrument_id = Uuid::new_v4();
let engine = MatchingEngine::with_event_bus(instrument_id, event_bus);
```

#### 5. Built-in Event Handlers

The engine comes with pre-built handlers:

- `EventLogger` - Keeps an in-memory log of recent events
- `PersistenceEventHandler` - Writes events to JSON files for durability

Example using the persistence handler:

```rust
use ultimate_matching::events::PersistenceEventHandler;

// Store events in the ./events directory, with 1000 events per file
let persistence_handler = PersistenceEventHandler::new("./events", 1000)?;
dispatcher.register_handler(Arc::new(persistence_handler)).await;
```

## Performance Considerations

The event system is designed to have minimal impact on the core matching operations:

- Events are published using tokio broadcast channels (lock-free)
- Event handlers run in separate async tasks
- The matching engine never blocks waiting for event handling
- High-frequency events like DepthUpdated can be filtered by handlers
- Event batching is used to reduce I/O overhead

## Example

A complete example is available in `src/main.rs`.

Run it with:

```bash
cargo run --release
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
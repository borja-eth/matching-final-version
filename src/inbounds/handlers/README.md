# Inbound Handlers Module

This module contains the request handlers that process incoming messages from external systems through RabbitMQ. These handlers form the entry point for all external commands into the matching engine.

## Purpose

The handlers in this module serve as the bridge between external message formats and the domain model of the matching engine:

1. Receive binary messages from RabbitMQ
2. Deserialize and validate the requests
3. Convert external DTOs to domain models
4. Forward requests to the OrderbookManagerService
5. Handle and log errors

## Module Structure

The module is organized into separate handler files, each responsible for a specific message type:

| Handler File | Purpose |
|-------------|---------|
| `cancel_handler.rs` | Processes order cancellation requests |
| `place_handler.rs` | Processes order placement requests |
| `snapshot_handler.rs` | Processes orderbook snapshot requests |
| `trading_status_handler.rs` | Processes trading status updates |
| `mod.rs` | Exports all handlers for external use |

## API Reference

```
+----------------------------------------------------------------------------------------+
|                             HANDLERS MODULE API REFERENCE                              |
+----------------------+-------------------+---------------------------------------------+
| FILE                 | TYPE              | ITEM                                        |
+----------------------+-------------------+---------------------------------------------+
| cancel_handler.rs    | Function          | handle_cancel_request(                      |
|                      |                   |   request: Vec<u8>,                         |
|                      |                   |   orderbook_manager: Arc<dyn OrderbookMan…> |
|                      |                   | )                                           |
+----------------------+-------------------+---------------------------------------------+
| place_handler.rs     | Function          | handle_place_request(                       |
|                      |                   |   request: Vec<u8>,                         |
|                      |                   |   orderbook_manager: Arc<dyn OrderbookMan…> |
|                      |                   | )                                           |
+----------------------+-------------------+---------------------------------------------+
| snapshot_handler.rs  | Function          | handle_snapshot_request(                    |
|                      |                   |   request: Vec<u8>,                         |
|                      |                   |   orderbook_manager: Arc<dyn OrderbookMan…> |
|                      |                   | )                                           |
+----------------------+-------------------+---------------------------------------------+
| trading_status_      | Function          | handle_trading_status_request(              |
| handler.rs           |                   |   request: Vec<u8>,                         |
|                      |                   |   orderbook_manager: Arc<dyn OrderbookMan…> |
|                      |                   | )                                           |
+----------------------+-------------------+---------------------------------------------+
| mod.rs               | Module Export     | pub mod cancel_handler;                     |
|                      | Module Export     | pub mod place_handler;                      |
|                      | Module Export     | pub mod snapshot_handler;                   |
|                      | Module Export     | pub mod trading_status_handler;             |
+----------------------+-------------------+---------------------------------------------+
|                                 RELATED DATA STRUCTURES                                |
+----------------------+-------------------+---------------------------------------------+
| ../dtos.rs           | Struct            | PlaceOrderRequest {                         |
|                      |                   |   version: u32,                             |
|                      |                   |   request_type: String,                     |
|                      |                   |   instrument: Uuid,                         |
|                      |                   |   ... // other fields                       |
|                      |                   | }                                           |
+----------------------+-------------------+---------------------------------------------+
| ../dtos.rs           | Struct            | CancelOrderRequest {                        |
|                      |                   |   version: u32,                             |
|                      |                   |   request_type: String,                     |
|                      |                   |   instrument: Uuid,                         |
|                      |                   |   order_id: Uuid,                           |
|                      |                   |   ... // other fields                       |
|                      |                   | }                                           |
+----------------------+-------------------+---------------------------------------------+
| ../dtos.rs           | Struct            | SnapshotRequest {                           |
|                      |                   |   instrument: Uuid,                         |
|                      |                   | }                                           |
+----------------------+-------------------+---------------------------------------------+
| ../dtos.rs           | Struct            | TradingStatusRequest {                      |
|                      |                   |   instrument: Uuid,                         |
|                      |                   | }                                           |
+----------------------+-------------------+---------------------------------------------+
| ../api_error.rs      | Enum              | ApiError {                                  |
|                      |                   |   BadRequest(String),                       |
|                      |                   |   InternalError(anyhow::Error),             |
|                      |                   | }                                           |
+----------------------+-------------------+---------------------------------------------+
|                            EXTERNAL DEPENDENCIES AND TRAITS                            |
+----------------------+-------------------+---------------------------------------------+
| domain/services/     | Trait             | OrderbookManagerService: Send + Sync {      |
| orderbook_manager    |                   |   fn add_order(&self, order: Order)         |
|                      |                   |     -> Result<(), OrderbookManagerError>;   |
|                      |                   |   fn cancel_order(                          |
|                      |                   |     &self,                                  |
|                      |                   |     instrument_id: &Uuid,                   |
|                      |                   |     order_id: Uuid                          |
|                      |                   |   ) -> Result<(), OrderbookManagerError>;   |
|                      |                   |   ... // other methods                      |
|                      |                   | }                                           |
+----------------------+-------------------+---------------------------------------------+
| domain/models/order  | Struct            | Order {                                     |
|                      |                   |   id: Uuid,                                 |
|                      |                   |   account_id: Uuid,                         |
|                      |                   |   side: OrderSide,                          |
|                      |                   |   ... // other fields                       |
|                      |                   | }                                           |
+----------------------+-------------------+---------------------------------------------+
| Standard Library     | Type              | Arc<T>                                      |
+----------------------+-------------------+---------------------------------------------+
| External Crate       | Function          | serde_json::from_slice<T>                   |
+----------------------+-------------------+---------------------------------------------+
| External Crate       | Module            | log                                         |
+----------------------+-------------------+---------------------------------------------+
```

## Handler Implementations

### Cancel Handler

Located in `cancel_handler.rs`, this handler processes order cancellation requests:

```rust
pub fn handle_cancel_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) {
    // Parse the cancel request
    // Forward to the orderbook manager
    // Handle errors
}
```

Key responsibilities:
- Deserializes `CancelOrderRequest` from binary data
- Logs cancellation attempts
- Calls `orderbook_manager.cancel_order()` with the appropriate instrument and order ID
- Handles and logs parsing errors and orderbook manager errors

### Place Handler

Located in `place_handler.rs`, this handler processes new order placement requests:

```rust
pub fn handle_place_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) {
    // Parse the place request
    // Convert to domain Order
    // Forward to the orderbook manager
    // Handle errors
}
```

Key responsibilities:
- Deserializes `PlaceOrderRequest` from binary data
- Converts the request to a domain `Order` model
- Logs order placement attempts
- Calls `orderbook_manager.add_order()` with the domain order
- Handles and logs parsing errors and orderbook manager errors

### Snapshot Handler

Located in `snapshot_handler.rs`, this handler processes orderbook snapshot requests:

```rust
pub fn handle_snapshot_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) {
    // Parse the snapshot request
    // Request snapshot from the orderbook manager
    // Handle errors
}
```

Key responsibilities:
- Deserializes `SnapshotRequest` from binary data
- Calls `orderbook_manager.publish_orderbook_snapshot()` with the instrument ID
- Handles and logs parsing errors and orderbook manager errors

### Trading Status Handler

Located in `trading_status_handler.rs`, this handler processes trading status update requests:

```rust
pub fn handle_trading_status_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) {
    // Parse the trading status request
    // Forward to the orderbook manager
    // Handle errors
}
```

Key responsibilities:
- Deserializes `TradingStatusRequest` from binary data
- Calls `orderbook_manager.publish_orderbook_status()` with the instrument ID
- Handles and logs parsing errors and orderbook manager errors

## Common Patterns

All handlers follow a consistent pattern:

1. Receive binary data (`Vec<u8>`) and a reference to the orderbook manager
2. Attempt to deserialize the request using `serde_json`
3. Log the request details
4. Forward the request to the appropriate orderbook manager method
5. Log any errors that occur

Error handling is primarily done through logging, with errors being converted to the internal `ApiError` type.

## Integration with Other Components

The handlers are integrated with:

- **OrderbookManagerService**: All handlers depend on this service to handle actual business logic
- **RabbitMQ Subscriber**: The subscriber calls these handlers with incoming messages
- **DTOs**: The handlers deserialize incoming messages into DTO structs from `inbounds::dtos`
- **Domain Models**: Handlers convert DTOs to domain models like `Order`
- **ApiError**: Used for consistent error representation

## Performance Considerations

The handlers are designed for minimal overhead:
- Deserialization is done once and efficiently
- No additional allocations after deserialization
- Errors are logged but don't block processing of other messages
- No synchronous waiting for responses

## Improvement Opportunities

### Rust Best Practices

1. **Error Handling**
   - Replace simple logging with proper error propagation
   - Use the `?` operator for better error handling flow
   - Return `Result<T, E>` from handlers instead of logging and discarding errors
   - Consider using `thiserror` for more structured error types

```rust
// Current approach:
if let Err(e) = orderbook_manager.publish_orderbook_status(...) {
    log::error!("Error publishing orderbook status: {}", e);
}

// Improved approach:
pub fn handle_trading_status_request(...) -> Result<(), HandlerError> {
    let trading_status_request = serde_json::from_slice(&request)
        .map_err(|e| HandlerError::DeserializationError(e))?;
    
    orderbook_manager.publish_orderbook_status(trading_status_request.instrument)
        .map_err(|e| HandlerError::OrderbookManagerError(e))?;
    
    Ok(())
}
```

2. **Message Validation**
   - Add explicit validation of incoming messages
   - Use `validator` crate for declarative validation
   - Reject malformed or invalid requests early

3. **Idiomatic Rust Types**
   - Use `&[u8]` instead of `Vec<u8>` for read-only binary data
   - Consider using `Cow<'a, [u8]>` for flexible ownership semantics
   - Use newtype patterns for stronger type safety

4. **Structured Logging**
   - Use structured logging with context fields
   - Add request IDs for tracing requests through the system
   - Include timing information for performance monitoring

### High-Throughput Matching Engine Best Practices

1. **Zero-Copy Parsing**
   - Implement zero-copy deserialization for critical messages
   - Use `serde_json::from_reader` with a cursor to avoid copies
   - Consider binary formats like FlatBuffers, Cap'n Proto, or Protocol Buffers

2. **Message Prioritization**
   - Implement priority handling for critical messages (e.g., cancellations)
   - Add explicit queue priorities in the message handler
   - Consider separate queues for different message types

3. **Backpressure Handling**
   - Add backpressure mechanisms to protect against message floods
   - Implement rate limiting per message type or client
   - Add circuit breakers to protect downstream services

4. **Metrics and Monitoring**
   - Add latency tracking for each handler
   - Track message rates, error rates, and processing times
   - Implement health checks and readiness probes

5. **Batching**
   - Add support for processing message batches
   - Optimize for throughput under high load
   - Implement adaptive batching based on load

### Architectural Improvements

1. **Handler Registry**
   - Create a registry pattern for handlers
   - Use a trait-based approach for consistent handler interfaces
   - Implement dynamic dispatch based on message type

```rust
pub trait MessageHandler<T> {
    fn handle(&self, message: T, orderbook_manager: Arc<dyn OrderbookManagerService>) -> Result<(), HandlerError>;
}

// Handler registry
pub struct HandlerRegistry {
    handlers: HashMap<MessageType, Box<dyn MessageHandler<Vec<u8>>>>,
}
```

2. **Async Processing**
   - Convert handlers to async functions
   - Implement non-blocking I/O throughout
   - Use `tokio::spawn` for concurrent message processing

```rust
pub async fn handle_cancel_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) -> Result<(), HandlerError> {
    // Async implementation
}
```

3. **Command Pattern**
   - Implement the command pattern for all handler operations
   - Represent each operation as a separate command object
   - Enable undo/replay capabilities for critical operations

4. **Specialized Handlers for Critical Paths**
   - Create optimized fast-path handlers for critical operations
   - Add specialized high-performance code paths for order placement
   - Implement dedicated cancellation workflows

## Testing Strategies

To ensure handlers work correctly:

1. **Unit Tests**
   - Test each handler with mock OrderbookManagerService
   - Verify correct error handling for malformed messages
   - Test boundary conditions and edge cases

2. **Property-Based Testing**
   - Use property-based testing to verify handler invariants
   - Generate random valid and invalid messages to test robustness
   - Ensure handlers maintain system consistency

3. **Load Testing**
   - Test handlers under high message volumes
   - Verify latency and throughput characteristics
   - Ensure handlers can handle peak loads

4. **Integration Testing**
   - Test handlers with actual RabbitMQ in a controlled environment
   - Verify end-to-end message flow
   - Test failure modes and recovery 
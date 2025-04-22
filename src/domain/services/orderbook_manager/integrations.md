# Orderbook Component Integration

This document outlines how to properly integrate the various orderbook components in the system.

## Overview

The orderbook system consists of the following components:

1. **OrderBook** (`domain/services/orderbook/orderbook.rs`): The core data structure that maintains bid/ask orders and implements matching logic.

2. **OrderBookWorker** (`domain/services/orderbook/orderbook_worker.rs`): A worker that processes orderbook operations asynchronously in a dedicated thread.

3. **OrderbookManagerService** (`domain/services/orderbook_manager/orderbook_manager_service.rs`): A service that manages multiple orderbooks and routes orders to the appropriate one.

4. **DepthTracker** (`domain/services/orderbook/depth.rs`): Tracks and aggregates orderbook depth information for efficient querying.

## Integration Guidelines

### 1. Starting the Orderbook Manager

The `OrderbookManagerServiceImpl` should be initialized with a list of instrument IDs:

```rust
let instruments = vec![Uuid::new_v4(), Uuid::new_v4()];
let orderbook_manager = OrderbookManagerServiceImpl::new(instruments);
```

### 2. Adding Orders

Orders should be routed through the manager to maintain consistent state:

```rust
let order = Order {
    id: Uuid::new_v4(),
    instrument_id: instrument_id, // Match a registered instrument
    // ... other fields
};

match orderbook_manager.add_order(order) {
    Ok(_) => println!("Order added successfully"),
    Err(e) => println!("Failed to add order: {}", e),
}
```

### 3. Cancelling Orders

Orders can be cancelled by providing their ID and instrument ID:

```rust
match orderbook_manager.cancel_order(&instrument_id, order_id) {
    Ok(_) => println!("Order cancelled successfully"),
    Err(e) => println!("Failed to cancel order: {}", e),
}
```

### 4. Halting and Resuming Orderbooks

Orderbooks can be halted and resumed for regulatory or maintenance purposes:

```rust
// Halt specific orderbooks
orderbook_manager.halt_orderbooks(vec![instrument_id1, instrument_id2]);

// Resume previously halted orderbooks
orderbook_manager.resume_orderbooks(vec![instrument_id1]);
```

### 5. Snapshots and Status Updates

You can request orderbook snapshots and status updates:

```rust
// Get a snapshot of the current orderbook state
orderbook_manager.publish_orderbook_snapshot(instrument_id);

// Get the current status (halted/resumed) of an orderbook
orderbook_manager.publish_orderbook_status(instrument_id);
```

## Implementation Notes

1. The `OrderbookWorker` uses a dedicated thread per orderbook to maximize throughput.
2. The manager uses a fan-out architecture to route orders to the appropriate orderbook thread.
3. Locking is minimized to prevent contention, with most operations using read locks.
4. Events are published asynchronously to prevent blocking the critical path.

## Error Handling

All operations return a `Result` with appropriate error types:

- `OrderbookManagerError::InstrumentNotRegistered`: When trying to operate on an unregistered instrument
- `OrderbookManagerError::OrderbookHalted`: When trying to add orders to a halted orderbook
- `OrderbookManagerError::ChannelSendError`: When there are communication issues between components
- `OrderbookManagerError::OrderbookError`: When an underlying orderbook operation fails

## Shutdown

To properly shut down the orderbook manager:

```rust
match orderbook_manager.stop() {
    Ok(_) => println!("Orderbook manager shut down successfully"),
    Err(e) => println!("Error during shutdown: {}", e),
}
```

This ensures all threads are properly terminated and resources are released. 
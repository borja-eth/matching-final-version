# Events System

This module implements the outbound adapters for the matching engine's event system. The events system follows the publisher-subscriber pattern and is responsible for distributing events from the core domain to external systems.

## Architecture

The events system consists of several components:

1. **Event Producers** - Inbound adapters that publish events to the system
2. **Event Handlers** - Outbound adapters that process events
3. **Event Store** - Persistence layer for storing and retrieving events
4. **Event Bus** - Core infrastructure for routing events between producers and handlers

## Event Types

The system handles various event types related to order processing and market data:

- **Order Events**: Order added, matched, cancelled, status changes
- **Market Events**: Trades executed, depth updates, snapshots
- **Session Events**: Trading session status changes

## Outbound Adapters

These adapters handle events and forward them to external systems:

- **FileEventStore**: Persists events to files in JSON Lines format
- **WebsocketNotifier**: Sends events to websocket clients
- **EventLogger**: Logs events and optionally maintains an in-memory history
- **OrderEventHandler**: Specialized handler for order-related events
- **MarketEventHandler**: Specialized handler for market data events

## Usage

The events system is designed to be used with dependency injection:

1. Create appropriate event handlers
2. Initialize an EventProducer with these handlers
3. Pass the EventProducer to domain services that need to emit events

Example:

```rust
// Create event handlers
let order_handler = OrderEventHandler::new();
let market_handler = MarketEventHandler::new();

// Create an event producer
let event_producer = EventProducer::new(order_handler, market_handler);

// Initialize a matching engine with the event producer
let matching_engine = MatchingEngine::with_event_producer(instrument_id, event_producer);
``` 
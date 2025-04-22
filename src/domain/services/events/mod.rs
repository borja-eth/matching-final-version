//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements an event-driven architecture for the matching engine, allowing
// for non-blocking event emission and processing while maintaining high performance.
//
// | Component                | Description                                                |
// |--------------------------|-----------------------------------------------------------|
// | MatchingEngineEvent      | Enum representing all possible events in the system       |
// | EventBus                 | Central hub for publishing and subscribing to events      |
// | EventHandler             | Trait for components that can handle events               |
// | EventDispatcher          | Component that routes events to registered handlers       |
//--------------------------------------------------------------------------------------------------

mod event_types;
mod event_bus;
mod dispatcher;
mod handlers;

#[cfg(test)]
mod tests;

// Re-exports
pub use event_types::{MatchingEngineEvent, EventError, EventResult};
pub use event_bus::EventBus;
pub use dispatcher::EventDispatcher;
pub use handlers::{EventHandler, EventLogger, PersistenceEventHandler}; 
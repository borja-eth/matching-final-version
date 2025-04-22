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
// DEPRECATION NOTICE:
// This module is being replaced by the new hexagonal architecture event system.
// New code should use the inbounds::events::EventProducer and domain::services::event_manager
// instead of components from this module.
//--------------------------------------------------------------------------------------------------

mod event_types;
mod event_bus;
mod dispatcher;
mod handlers;

#[cfg(test)]
mod tests;


#[deprecated(
    since = "1.0.0", 
    note = "Use EventProducer from inbounds::events instead"
)]
pub use event_bus::EventBus;

#[deprecated(
    since = "1.0.0", 
    note = "Use DefaultEventManager from domain::services::event_manager instead"
)]
pub use dispatcher::EventDispatcher;

#[deprecated(
    since = "1.0.0", 
    note = "Use EventHandler from domain::services::event_manager::ports instead"
)]
pub use handlers::{EventHandler, EventLogger, PersistenceEventHandler};

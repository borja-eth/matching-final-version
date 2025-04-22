// Expose the modules
pub mod domain;
// These modules should be accessed through domain::services
// pub mod events;
// pub mod orderbook;
// pub mod matching_engine;

// Re-export key types for easier usage
pub use domain::models::types::{Order, Side, OrderType, OrderStatus, Trade, TimeInForce};
pub use domain::services::orderbook::orderbook::OrderBook;
pub use domain::services::orderbook::OrderbookError;
pub use domain::services::orderbook::depth::{DepthSnapshot, PriceLevel};
pub use domain::services::matching_engine::matching_engine::{
    MatchingEngine, MatchResult, MatchingError
};
pub use domain::services::events::{
    EventBus, EventDispatcher, EventHandler, EventError, EventResult, MatchingEngineEvent,
    PersistenceEventHandler, EventLogger
}; 

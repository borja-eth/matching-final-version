// Expose the modules
pub mod types;
pub mod orderbook;
pub mod matching_engine;
pub mod depth;
pub mod events;

// Re-export key types for easier usage
pub use types::{Order, Side, OrderType, OrderStatus, Trade, TimeInForce};
pub use orderbook::OrderBook;
pub use matching_engine::{MatchingEngine, MatchResult, MatchingError};
pub use events::bus::EventBus;
pub use events::types::Event; 
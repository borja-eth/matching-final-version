// Expose the modules
pub mod domain;
pub mod inbounds;
pub mod outbounds;
pub mod config;
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
pub use domain::services::orderbook_manager::{
    OrderbookManagerService, OrderbookManagerError
};
pub use domain::services::orderbook_manager::orderbook_manager_service::OrderbookManagerServiceImpl;

// Re-export inbound/outbound event types
pub use outbounds::events::order::OrderEventHandler;
pub use outbounds::events::market::MarketEventHandler; 

// Re-export Config
pub use config::Config;

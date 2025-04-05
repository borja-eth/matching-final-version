//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements a REST API using Axum for the matching engine.
// It provides endpoints for order management, market data, and system status.
//
// | Component      | Description                                                |
// |----------------|-----------------------------------------------------------|
// | API            | Main API structure coordinating routes and services        |
// | Routes         | Handler functions for API endpoints                        |
// | States         | Shared application state                                   |
// | DTOs           | Data transfer objects for API requests/responses           |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name           | Description                                       | Key Methods       |
// |----------------|---------------------------------------------------|------------------|
// | AppState       | Shared application state                         | new               |
// | Api            | Main API structure                               | serve             |
// | Error          | API error types                                  | from              |
//--------------------------------------------------------------------------------------------------

mod routes;
mod dto;
mod error;

use std::sync::Arc;
use std::net::SocketAddr;
use axum::{
    Router,
    Extension,
    routing::{get, post, delete},
    http::{Method, header, HeaderValue},
};
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use uuid::Uuid;
use tower_http::cors::CorsLayer;

use crate::matching_engine::MatchingEngine;
use crate::events::EventBus;

pub use error::{ApiError, ApiResult};
pub use dto::*;

/// Shared application state accessible by all handlers
pub struct AppState {
    /// Map of instrument ID to matching engine instance
    pub engines: Arc<RwLock<std::collections::HashMap<Uuid, Arc<RwLock<MatchingEngine>>>>>,
    /// Shared event bus
    pub event_bus: Arc<EventBus>,
}

impl AppState {
    /// Creates a new application state
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            engines: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_bus: Arc::new(event_bus),
        }
    }
    
    /// Adds a new instrument and creates a matching engine for it
    pub async fn add_instrument(&self, instrument_id: Uuid) {
        let mut engines = self.engines.write().await;
        if !engines.contains_key(&instrument_id) {
            // Clone the inner EventBus from the Arc wrapper
            let engine = MatchingEngine::with_event_bus(instrument_id, (*self.event_bus).clone());
            engines.insert(instrument_id, Arc::new(RwLock::new(engine)));
        }
    }
    
    /// Gets a matching engine for an instrument
    pub async fn get_engine(&self, instrument_id: &Uuid) -> Option<Arc<RwLock<MatchingEngine>>> {
        let engines = self.engines.read().await;
        engines.get(instrument_id).cloned()
    }
}

/// Main API structure
pub struct Api {
    /// API address
    addr: SocketAddr,
    /// Shared application state
    state: Arc<AppState>,
}

impl Api {
    /// Creates a new API instance
    pub fn new(addr: SocketAddr, event_bus: EventBus) -> Self {
        let state = Arc::new(AppState::new(event_bus));
        Self { addr, state }
    }
    
    /// Creates all routes for the API
    pub fn routes(&self) -> Router {
        // Create a CORS layer that allows requests from specific origins
        let cors = CorsLayer::new()
            // Allow requests from localhost origins
            .allow_origin([
                "http://localhost:3000".parse::<HeaderValue>().unwrap(),
                "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
                "http://localhost:3001".parse::<HeaderValue>().unwrap(),
                "http://127.0.0.1:3001".parse::<HeaderValue>().unwrap(),
            ])
            // Allow standard methods
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            // Allow specific headers
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
            ])
            // Allow credentials
            .allow_credentials(true);
            
        Router::new()
            // Health check
            .route("/health", get(routes::health))
            
            // Order management
            .route("/orders", post(routes::create_order))
            .route("/orders/:id", delete(routes::cancel_order))
            .route("/orders/:id", get(routes::get_order))
            
            // Market data
            .route("/instruments/:id/orderbook", get(routes::get_orderbook))
            .route("/instruments/:id/depth", get(routes::get_depth))
            .route("/instruments/:id/trades", get(routes::get_trades))
            
            // System management
            .route("/instruments", post(routes::create_instrument))
            .route("/instruments", get(routes::list_instruments))
            
            // Attach application state
            .layer(Extension(self.state.clone()))
            // Add CORS layer
            .layer(cors)
    }
    
    /// Starts the API server and runs until shutdown
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.routes();
        
        println!("API listening on {}", self.addr);
        // Create a TcpListener first, then pass it to axum::serve
        let listener = TcpListener::bind(self.addr).await?;
        axum::serve(listener, app).await?;
            
        Ok(())
    }
} 
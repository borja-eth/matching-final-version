//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | MatchingEngineServiceImpl | Concrete implementation of MatchingEngineService | process_order    |
// |                         |                                                   | cancel_order     |
// |                         |                                                   | get_depth        |
//--------------------------------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::services::matching_engine::{
    MatchingEngineService, 
    MatchingEngineServiceResult, 
    MatchingEngineServiceError,
    matching_engine::MatchingEngine
};
use crate::domain::services::matching_engine::matching_engine::MatchResult;
use crate::domain::models::types::{Order, TimeInForce};
use crate::domain::services::orderbook::depth::DepthSnapshot;

/// Concrete implementation of the MatchingEngineService trait
pub struct MatchingEngineServiceImpl {
    /// Map of instrument ID to matching engine
    engines: HashMap<Uuid, Arc<RwLock<MatchingEngine>>>,
}

impl MatchingEngineService for MatchingEngineServiceImpl {
    /// Creates a new instance of the matching engine service
    fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }
    
    /// Process an order through the appropriate matching engine
    fn process_order(&mut self, order: Order) -> MatchingEngineServiceResult<MatchResult> {
        let instrument_id = order.instrument_id;
        
        // Get the matching engine for this instrument
        match self.engines.get(&instrument_id) {
            Some(engine) => {
                // We can't do async operations in this sync method, so we'd need a different approach
                // in a real implementation. This is just for demonstration.
                Err(MatchingEngineServiceError::Other(
                    "Sync API can't access async engine lock - use async API instead".to_string()
                ))
            },
            None => Err(MatchingEngineServiceError::EngineNotFound(instrument_id)),
        }
    }
    
    /// Cancel an existing order in the appropriate matching engine
    fn cancel_order(&mut self, order_id: Uuid, instrument_id: Uuid) -> MatchingEngineServiceResult<Order> {
        // Similar limitations as process_order
        match self.engines.get(&instrument_id) {
            Some(_) => {
                Err(MatchingEngineServiceError::Other(
                    "Sync API can't access async engine lock - use async API instead".to_string()
                ))
            },
            None => Err(MatchingEngineServiceError::EngineNotFound(instrument_id)),
        }
    }
    
    /// Get the order book depth for a specific instrument
    fn get_depth(&mut self, instrument_id: Uuid, _levels: usize) -> MatchingEngineServiceResult<DepthSnapshot> {
        // Similar limitations as other methods
        match self.engines.get(&instrument_id) {
            Some(_) => {
                Err(MatchingEngineServiceError::Other(
                    "Sync API can't access async engine lock - use async API instead".to_string()
                ))
            },
            None => Err(MatchingEngineServiceError::EngineNotFound(instrument_id)),
        }
    }
} 
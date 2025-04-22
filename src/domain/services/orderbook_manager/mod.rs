use thiserror::Error;
use uuid::Uuid;

use crate::domain::models::types::Order;

use super::orderbook::OrderbookError;

pub mod orderbook_manager_service;

/// Service for managing multiple orderbooks across different instruments.
/// 
/// This trait defines the interface for interacting with order routing,
/// orderbook lifecycle management (halting/resuming), and status publishing.
/// Implementations must be thread-safe to support concurrent access.
pub trait OrderbookManagerService: Send + Sync {
    /// Routes an order to the appropriate orderbook for processing.
    ///
    /// # Arguments
    /// * `order` - The order to be added to an orderbook
    ///
    /// # Returns
    /// * `Ok(())` - If the order was successfully routed
    /// * `Err(OrderbookManagerError)` - If routing failed
    fn add_order(&self, order: Order) -> Result<(), OrderbookManagerError>;
    
    /// Cancels an existing order in the specified orderbook.
    ///
    /// # Arguments
    /// * `instrument_id` - The instrument identifier for the orderbook
    /// * `order_id` - The unique identifier of the order to cancel
    ///
    /// # Returns
    /// * `Ok(())` - If the cancel request was successfully routed
    /// * `Err(OrderbookManagerError)` - If routing failed
    fn cancel_order(
        &self,
        instrument_id: &Uuid,
        order_id: Uuid,
    ) -> Result<(), OrderbookManagerError>;
    
    /// Halts trading for the specified instruments.
    ///
    /// # Arguments
    /// * `instruments` - List of instrument IDs to halt
    fn halt_orderbooks(&mut self, instruments: Vec<Uuid>);
    
    /// Resumes trading for previously halted instruments.
    ///
    /// # Arguments
    /// * `instruments` - List of instrument IDs to resume
    fn resume_orderbooks(&mut self, instruments: Vec<Uuid>);
    
    /// Publishes the current status (halted/resumed) of an orderbook.
    ///
    /// # Arguments
    /// * `instrument_id` - The instrument identifier
    ///
    /// # Returns
    /// * `Ok(())` - If status was published successfully
    /// * `Err(OrderbookManagerError)` - If publishing failed
    fn publish_orderbook_status(&self, instrument_id: Uuid) -> Result<(), OrderbookManagerError>;
    
    /// Triggers generation of a full orderbook snapshot.
    ///
    /// # Arguments
    /// * `instrument_id` - The instrument identifier
    ///
    /// # Returns
    /// * `Ok(())` - If snapshot request was routed successfully
    /// * `Err(OrderbookManagerError)` - If request failed
    fn publish_orderbook_snapshot(&self, instrument_id: Uuid) -> Result<(), OrderbookManagerError>;
    
    /// Initializes and starts the orderbook manager service.
    ///
    /// # Returns
    /// * `Ok(())` - If service started successfully
    /// * `Err(OrderbookManagerError)` - If service failed to start
    fn start(&self) -> Result<(), OrderbookManagerError>;
}

/// Errors that can occur during orderbook manager operations.
#[derive(Debug, Error)]
pub enum OrderbookManagerError {
    /// The requested instrument is not registered with the manager.
    #[error("Instrument not registered: {0}")]
    InstrumentNotRegistered(Uuid),
    
    /// Error sending a message through a channel.
    #[error("Channel send error: {0}")]
    ChannelSendError(anyhow::Error),
    
    /// Error from the underlying orderbook.
    #[error("Orderbook error: {0}")]
    OrderbookError(#[from] OrderbookError),
    
    /// Operation timed out.
    #[error("Timeout occurred")]
    Timeout,
    
    /// Error closing an orderbook.
    #[error("Close orderbook error: {0}")]
    CloseOrderbookError(String),
    
    /// Operation rejected because orderbook is halted.
    #[error("Orderbook halted: {0}")]
    OrderbookHalted(Uuid),
}

//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module contains the orderbook manager service, which is responsible for managing multiple
// orderbooks for different instruments.
//--------------------------------------------------------------------------------------------------

/// +----------------------------------------------------------+
/// | MODULES                                                  |
/// +----------+-------+-------+------------------------------+
/// | Exports:                                                 |
/// |   - OrderbookManagerService (trait)                      |
/// |   - OrderbookManagerServiceImpl (struct)                 |
/// |   - OrderbookManagerError (enum)                         |
/// |   - MockOrderbookManagerService (for tests)              |
/// +----------------------------------------------------------+

#[cfg(test)]
use mockall::*;

#[cfg(test)]
mock! {
    pub OrderbookManagerService {}
    
    impl OrderbookManagerService for OrderbookManagerService {
        fn add_order(&self, order: crate::domain::models::types::Order) -> Result<(), OrderbookManagerError>;
        
        fn cancel_order(&self, instrument_id: &uuid::Uuid, order_id: uuid::Uuid) -> Result<(), OrderbookManagerError>;
        
        fn halt_orderbooks(&mut self, instruments: Vec<uuid::Uuid>);
        
        fn resume_orderbooks(&mut self, instruments: Vec<uuid::Uuid>);
        
        fn publish_orderbook_status(&self, instrument_id: uuid::Uuid) -> Result<(), OrderbookManagerError>;
        
        fn publish_orderbook_snapshot(&self, instrument_id: uuid::Uuid) -> Result<(), OrderbookManagerError>;
        
        fn start(&self) -> Result<(), OrderbookManagerError>;
    }
}

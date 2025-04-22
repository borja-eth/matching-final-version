//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements the event system using hexagonal architecture principles.
// It separates core domain logic from external concerns through ports and adapters.
//
// Core components:
// - Domain ports: Interfaces for how the application interacts with events
// - Inbound ports: Interfaces for external components to publish events to the system
// - Outbound ports: Interfaces for the system to notify external components about events
//
// The module structure follows the hexagonal architecture pattern:
// - Core domain contains the business logic and port interfaces
// - Inbound adapters implement inbound ports for input into the system
// - Outbound adapters implement outbound ports for output from the system
//--------------------------------------------------------------------------------------------------

// Service implementation
pub mod event_manager_service;

use std::thread;
use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::domain::models::orderbook::OrderbookResult;

/// Errors that can occur in the event manager service
#[derive(Error, Debug)]
pub enum EventManagerError {
    /// Error when publishing events to RabbitMQ
    #[error("Error publishing event: {0}")]
    PublishError(String),
    
    /// Error when serializing events
    #[error("Error serializing event: {0}")]
    SerializationError(String),
    
    /// Error in RabbitMQ connection
    #[error("RabbitMQ connection error: {0}")]
    RabbitMQError(String),
}

/// Interface for the event manager service
#[async_trait]
pub trait EventManagerService: Send + Sync {
    /// Starts the event manager service
    ///
    /// # Returns
    /// Result indicating success or an error
    async fn start(&self) -> Result<(), EventManagerError>;
    
    /// Runs the event processing loop in a separate thread
    ///
    /// # Arguments
    /// * `result_receiver` - Channel receiving orderbook results tagged with instrument ID
    ///
    /// # Returns
    /// JoinHandle for the spawned thread
    fn run(&self, result_receiver: Receiver<(Uuid, OrderbookResult)>) -> thread::JoinHandle<()>;
} 
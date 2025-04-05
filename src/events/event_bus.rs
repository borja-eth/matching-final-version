//--------------------------------------------------------------------------------------------------
// STRUCTS & TRAITS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | EventBus                | Central event publishing component               | publish, subscribe|
//--------------------------------------------------------------------------------------------------

use tokio::sync::broadcast;
use tracing::{debug, error};

use super::event_types::{MatchingEngineEvent, EventError, EventResult};

/// Central event bus for publishing and subscribing to events
#[derive(Debug, Clone)]
pub struct EventBus {
    /// Channel for broadcasting events to all subscribers
    sender: broadcast::Sender<MatchingEngineEvent>,
    /// Capacity of the event channel
    capacity: usize,
}

impl EventBus {
    /// Creates a new event bus with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            capacity,
        }
    }
    
    /// Creates a new event bus with default capacity
    pub fn default() -> Self {
        Self::new(1024) // Default capacity of 1024 events
    }
    
    /// Publishes an event to all subscribers
    pub fn publish(&self, event: MatchingEngineEvent) -> EventResult<()> {
        debug!("Publishing event: {:?}", event);
        
        // If there are no receivers, this is a no-op (not an error)
        if self.sender.receiver_count() == 0 {
            debug!("No subscribers for event: {:?}", event);
            return Ok(());
        }
        
        match self.sender.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to publish event: {}", e);
                Err(EventError::PublishError(e.to_string()))
            }
        }
    }
    
    /// Creates a new subscription to receive events
    pub fn subscribe(&self) -> broadcast::Receiver<MatchingEngineEvent> {
        self.sender.subscribe()
    }
    
    /// Returns the current number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
    
    /// Returns the capacity of the event channel
    pub fn capacity(&self) -> usize {
        self.capacity
    }
} 
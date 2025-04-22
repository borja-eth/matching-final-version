//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements a central event bus for the matching engine using tokio's broadcast channel.
// It provides a way for components to publish and subscribe to events throughout the system.
//
// | Component     | Description                                                 |
// |---------------|-------------------------------------------------------------|
// | EventBus      | Central event bus for publishing and subscribing to events  |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name          | Description                                   | Key Methods                   |
// |---------------|-----------------------------------------------|-------------------------------|
// | EventBus      | Central event publishing component           | publish, subscribe            |
//
//--------------------------------------------------------------------------------------------------

use tokio::sync::broadcast;
use tracing::{debug, error, info};

use super::event_types::{MatchingEngineEvent, EventError, EventResult};

/// Central event bus for publishing and subscribing to events throughout the system.
/// 
/// The `EventBus` uses tokio's broadcast channel to efficiently deliver events
/// to multiple subscribers with minimal overhead. Events are distributed to all
/// active subscribers when published.
///
/// # Examples
///
/// ```
/// use crate::domain::services::events::{EventBus, MatchingEngineEvent};
///
/// // Create a new event bus with default capacity
/// let event_bus = EventBus::default();
///
/// // Subscribe to events
/// let mut subscriber = event_bus.subscribe();
///
/// // Publish an event
/// event_bus.publish(MatchingEngineEvent::Initialized).unwrap();
///
/// // In a separate task or thread, receive the event
/// tokio::spawn(async move {
///     match subscriber.recv().await {
///         Ok(event) => println!("Received event: {:?}", event),
///         Err(err) => println!("Error receiving event: {}", err),
///     }
/// });
/// ```
#[derive(Debug, Clone)]
pub struct EventBus {
    /// Channel for broadcasting events to all subscribers
    sender: broadcast::Sender<MatchingEngineEvent>,
    /// Capacity of the event channel
    capacity: usize,
}

impl EventBus {
    /// Creates a new event bus with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The maximum number of events that can be queued before
    ///   older events are dropped.
    ///
    /// # Returns
    ///
    /// A new `EventBus` instance with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        info!("Created new EventBus with capacity: {}", capacity);
        Self {
            sender,
            capacity,
        }
    }
    
    /// Creates a new event bus with default capacity of 1024 events.
    ///
    /// # Returns
    ///
    /// A new `EventBus` instance with the default capacity.
    pub fn default() -> Self {
        Self::new(1024)
    }
    
    /// Publishes an event to all subscribers.
    ///
    /// If there are no subscribers, the event is simply dropped and
    /// the method returns successfully.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to publish to all subscribers.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the event was successfully published or there were no subscribers.
    /// * `Err(EventError)` - If there was an error publishing the event.
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
    
    /// Creates a new subscription to receive events.
    ///
    /// # Returns
    ///
    /// A `broadcast::Receiver` that can be used to receive events published to this bus.
    /// Multiple calls to this method will create multiple independent subscriptions.
    pub fn subscribe(&self) -> broadcast::Receiver<MatchingEngineEvent> {
        debug!("New subscriber added to EventBus (total: {})", self.sender.receiver_count() + 1);
        self.sender.subscribe()
    }
    
    /// Returns the current number of subscribers.
    ///
    /// # Returns
    ///
    /// The number of active subscribers to this event bus.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
    
    /// Returns the capacity of the event channel.
    ///
    /// # Returns
    ///
    /// The maximum number of events that can be queued in the channel.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
} 
//--------------------------------------------------------------------------------------------------
// STRUCTS & TRAITS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | EventDispatcher         | Routes events to registered handlers             | dispatch, start   |
//--------------------------------------------------------------------------------------------------

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use super::event_types::MatchingEngineEvent;
use super::event_bus::EventBus;
use super::handlers::EventHandler;

/// Dispatches events to registered handlers
pub struct EventDispatcher {
    /// Event bus for receiving events
    event_bus: EventBus,
    /// Map of event types to handlers
    handlers: Arc<RwLock<HashMap<&'static str, Vec<Arc<dyn EventHandler>>>>>,
    /// Buffer size for event processing
    buffer_size: usize,
}

// Manually implement Debug for EventDispatcher to handle the non-Debug trait objects
impl fmt::Debug for EventDispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventDispatcher")
            .field("buffer_size", &self.buffer_size)
            .field("event_bus", &self.event_bus)
            .field("handlers", &format!("[{} handler types]", 
                // Just show count of handler types, not the actual handlers
                // This is async code, so we can't actually access the handlers here
                // as it would require a blocking read lock
                "..."
            ))
            .finish()
    }
}

impl EventDispatcher {
    /// Creates a new event dispatcher with default buffer size.
    ///
    /// # Arguments
    /// * `event_bus` - The event bus to subscribe to for events
    ///
    /// # Returns
    /// A new `EventDispatcher` instance
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            event_bus,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            buffer_size: 100, // Default buffer size
        }
    }
    
    /// Registers a handler for processing specific event types.
    ///
    /// # Arguments
    /// * `handler` - The event handler to register
    ///
    /// The handler will only receive events that match the types it declares
    /// in its `event_types()` method.
    pub async fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        let mut handlers = self.handlers.write().await;
        
        // Register for each event type the handler can process
        for event_type in handler.event_types() {
            handlers.entry(event_type).or_insert_with(Vec::new).push(Arc::clone(&handler));
        }
        
        debug!(
            "Registered handler for event types: {:?}", 
            handler.event_types()
        );
    }
    
    /// Starts the dispatcher to process events in the background.
    ///
    /// # Returns
    /// A JoinHandle that can be used to await the dispatcher's completion
    pub async fn start(self) -> tokio::task::JoinHandle<()> {
        let handlers = Arc::clone(&self.handlers);
        let mut receiver = self.event_bus.subscribe();
        let buffer_size = self.buffer_size;
        
        // Spawn a task to process events
        tokio::spawn(async move {
            info!("Event dispatcher started");
            
            // Create a buffer for batching event processing
            let (tx, mut rx) = mpsc::channel(buffer_size);
            
            // Spawn a task to receive events and send them to the buffer
            let receiver_task = tokio::spawn(async move {
                while let Ok(event) = receiver.recv().await {
                    if let Err(e) = tx.send(event).await {
                        error!("Failed to send event to processing buffer: {}", e);
                        break;
                    }
                }
            });
            
            // Process events from the buffer
            while let Some(event) = rx.recv().await {
                let event_type = match &event {
                    MatchingEngineEvent::OrderAdded { .. } => "OrderAdded",
                    MatchingEngineEvent::OrderMatched { .. } => "OrderMatched",
                    MatchingEngineEvent::OrderCancelled { .. } => "OrderCancelled",
                    MatchingEngineEvent::OrderStatusChanged { .. } => "OrderStatusChanged",
                    MatchingEngineEvent::TradeExecuted { .. } => "TradeExecuted",
                    MatchingEngineEvent::DepthUpdated { .. } => "DepthUpdated",
                };
                
                let handlers_lock = handlers.read().await;
                if let Some(event_handlers) = handlers_lock.get(event_type) {
                    for handler in event_handlers {
                        let handler = Arc::clone(handler);
                        let event_clone = event.clone();
                        
                        // Spawn a task to handle the event
                        tokio::spawn(async move {
                            if let Err(e) = handler.handle_event(event_clone).await {
                                error!("Handler failed to process event: {}", e);
                            }
                        });
                    }
                } else {
                    debug!("No handlers registered for event type: {}", event_type);
                }
            }
            
            // Log error if receiver task failed, but continue with shutdown
            if let Err(e) = receiver_task.await {
                error!("Receiver task failed: {}", e);
            }
            
            info!("Event dispatcher stopped");
        })
    }
    
    /// Sets the buffer size for event processing.
    ///
    /// # Arguments
    /// * `buffer_size` - The size of the internal event buffer
    ///
    /// # Returns
    /// Self with updated buffer size for method chaining
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }
} 
//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements an event-driven architecture for the matching engine, allowing
// for non-blocking event emission and processing while maintaining high performance.
//
// | Component                | Description                                                |
// |--------------------------|-----------------------------------------------------------|
// | MatchingEngineEvent      | Enum representing all possible events in the system       |
// | EventBus                 | Central hub for publishing and subscribing to events      |
// | EventHandler             | Trait for components that can handle events               |
// | EventDispatcher          | Component that routes events to registered handlers       |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS & TRAITS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | MatchingEngineEvent     | Event variants for the matching engine           | clone, send, sync |
// | EventBus                | Central event publishing component               | publish, subscribe|
// | EventHandler            | Trait for event handling                         | handle_event      |
// | EventDispatcher         | Routes events to registered handlers             | dispatch          |
//
//--------------------------------------------------------------------------------------------------
// FUNCTIONS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Return Type      |
// |-------------------------|---------------------------------------------------|------------------|
// | publish                 | Publishes an event to all subscribers            | Result<()>       |
// | subscribe               | Subscribes to receive events                     | Receiver         |
// | handle_event            | Processes a specific event                       | Future<Result>   |
// | dispatch                | Routes an event to appropriate handlers          | Future<Result>   |
//--------------------------------------------------------------------------------------------------

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use chrono::Utc;
use serde::{Serialize, Deserialize};
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::types::{Order, Trade, Side};
use crate::depth::DepthSnapshot;

/// Errors that can occur in the event system
#[derive(Error, Debug, Clone)]
pub enum EventError {
    /// Failed to publish an event (e.g., no subscribers or channel full)
    #[error("Failed to publish event: {0}")]
    PublishError(String),
    
    /// Failed to process an event
    #[error("Failed to process event: {0}")]
    ProcessingError(String),
    
    /// Event handler not found for event type
    #[error("No handler registered for event type: {0}")]
    HandlerNotFound(String),
}

/// Type alias for Result with EventError
pub type EventResult<T> = Result<T, EventError>;

/// Represents events that can occur in the matching engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchingEngineEvent {
    /// Generated when an order is added to the book
    OrderAdded {
        /// The order that was added
        order: Order,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when an order is matched (partially or fully)
    OrderMatched {
        /// The order that was matched
        order: Order,
        /// Amount of the order that was matched
        matched_quantity: rust_decimal::Decimal,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when an order is cancelled
    OrderCancelled {
        /// The order that was cancelled
        order: Order,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when an order's status changes
    OrderStatusChanged {
        /// The order ID
        order_id: Uuid,
        /// Previous status
        previous_status: crate::types::OrderStatus,
        /// New status
        new_status: crate::types::OrderStatus,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when a trade is executed
    TradeExecuted {
        /// The trade that was executed
        trade: Trade,
        /// Timestamp when the event occurred
        timestamp: chrono::DateTime<Utc>,
    },
    
    /// Generated when the depth is updated
    DepthUpdated {
        /// The updated depth snapshot
        depth: DepthSnapshot,
        /// Timestamp when the event occurred 
        timestamp: chrono::DateTime<Utc>,
    },
}

/// Event handler trait for processing events
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    /// Returns the types of events this handler processes
    fn event_types(&self) -> Vec<&'static str>;
    
    /// Processes an event
    async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()>;
}

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

/// Dispatches events to registered handlers
pub struct EventDispatcher {
    /// Event bus for receiving events
    event_bus: EventBus,
    /// Map of event types to handlers
    handlers: Arc<RwLock<HashMap<&'static str, Vec<Arc<dyn EventHandler>>>>>,
    /// Buffer for event processing
    buffer_size: usize,
}

impl EventDispatcher {
    /// Creates a new event dispatcher
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            event_bus,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            buffer_size: 100, // Default buffer size
        }
    }
    
    /// Registers a handler for processing events
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
    
    /// Starts the dispatcher to process events
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
            
            if let Err(e) = receiver_task.await {
                error!("Receiver task failed: {}", e);
            }
            
            info!("Event dispatcher stopped");
        })
    }
    
    /// Sets the buffer size for event processing
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }
}

/// A simple in-memory event logger for debugging
pub struct EventLogger {
    /// Maximum number of events to keep in history
    max_history: usize,
    /// Event history
    history: Arc<RwLock<Vec<MatchingEngineEvent>>>,
}

impl EventLogger {
    /// Creates a new event logger
    pub fn new(max_history: usize) -> Self {
        Self {
            max_history,
            history: Arc::new(RwLock::new(Vec::with_capacity(max_history))),
        }
    }
    
    /// Returns the event history
    pub async fn get_history(&self) -> Vec<MatchingEngineEvent> {
        self.history.read().await.clone()
    }
}

#[async_trait::async_trait]
impl EventHandler for EventLogger {
    fn event_types(&self) -> Vec<&'static str> {
        vec![
            "OrderAdded", 
            "OrderMatched", 
            "OrderCancelled", 
            "OrderStatusChanged",
            "TradeExecuted", 
            "DepthUpdated"
        ]
    }
    
    async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()> {
        let mut history = self.history.write().await;
        
        // Remove oldest event if at capacity
        if history.len() >= self.max_history {
            history.remove(0);
        }
        
        // Add new event
        history.push(event);
        
        Ok(())
    }
}

// A persistence-oriented event handler that writes events to a JSON file
pub struct PersistenceEventHandler {
    /// Directory to store event files
    output_dir: std::path::PathBuf,
    /// File handle for current write operations
    current_file: tokio::sync::Mutex<Option<tokio::fs::File>>,
    /// Maximum events per file before rotation
    max_events_per_file: usize,
    /// Current event count in the current file
    event_count: std::sync::atomic::AtomicUsize,
}

impl PersistenceEventHandler {
    /// Creates a new persistence handler
    pub fn new<P: AsRef<std::path::Path>>(output_dir: P, max_events_per_file: usize) -> std::io::Result<Self> {
        let path = output_dir.as_ref().to_path_buf();
        
        // Ensure directory exists
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        
        Ok(Self {
            output_dir: path,
            current_file: tokio::sync::Mutex::new(None),
            max_events_per_file,
            event_count: std::sync::atomic::AtomicUsize::new(0),
        })
    }
    
    /// Opens a new file for writing events
    async fn open_new_file(&self) -> std::io::Result<tokio::fs::File> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let filename = format!("events_{}.jsonl", timestamp);
        let path = self.output_dir.join(filename);
        
        debug!("Opening new event file: {:?}", path);
        
        let file = tokio::fs::File::create(path).await?;
        self.event_count.store(0, std::sync::atomic::Ordering::SeqCst);
        
        Ok(file)
    }
    
    /// Writes an event to the current file
    async fn write_event(&self, event: &MatchingEngineEvent) -> std::io::Result<()> {
        let mut file_guard = self.current_file.lock().await;
        
        // Create file if it doesn't exist or needs rotation
        if file_guard.is_none() || self.event_count.load(std::sync::atomic::Ordering::SeqCst) >= self.max_events_per_file {
            *file_guard = Some(self.open_new_file().await?);
        }
        
        // Get the file
        let file = file_guard.as_mut().unwrap();
        
        // Serialize event to JSON
        let json = serde_json::to_string(&event)?;
        
        // Write to file with newline
        use tokio::io::AsyncWriteExt;
        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        
        // Increment event count
        self.event_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler for PersistenceEventHandler {
    fn event_types(&self) -> Vec<&'static str> {
        vec![
            "OrderAdded", 
            "OrderMatched", 
            "OrderCancelled", 
            "OrderStatusChanged",
            "TradeExecuted", 
            // We exclude DepthUpdated as it would generate too many events
        ]
    }
    
    async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()> {
        // Skip depth updates to reduce storage requirements
        if let MatchingEngineEvent::DepthUpdated { .. } = event {
            return Ok(());
        }
        
        // Write event to file
        match self.write_event(&event).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to persist event: {}", e);
                Err(EventError::ProcessingError(format!("Failed to persist event: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderType, OrderStatus, CreatedFrom};
    use rust_decimal_macros::dec;
    
    // Helper to create a test order
    fn create_test_order() -> Order {
        let now = Utc::now();
        Order {
            id: Uuid::new_v4(),
            ext_id: Some("test-order".to_string()),
            account_id: Uuid::new_v4(),
            order_type: OrderType::Limit,
            instrument_id: Uuid::new_v4(),
            side: Side::Bid,
            limit_price: Some(dec!(100.0)),
            trigger_price: None,
            base_amount: dec!(1.0),
            remaining_base: dec!(1.0),
            filled_quote: dec!(0.0),
            filled_base: dec!(0.0),
            remaining_quote: dec!(100.0),
            expiration_date: now + chrono::Duration::days(365),
            status: OrderStatus::New,
            created_at: now,
            updated_at: now,
            trigger_by: None,
            created_from: CreatedFrom::Api,
            sequence_id: 1,
        }
    }
    
    // Helper to create a test trade
    fn create_test_trade() -> Trade {
        Trade {
            id: Uuid::new_v4(),
            instrument_id: Uuid::new_v4(),
            maker_order_id: Uuid::new_v4(),
            taker_order_id: Uuid::new_v4(),
            base_amount: dec!(1.0),
            quote_amount: dec!(100.0),
            price: dec!(100.0),
            created_at: Utc::now(),
        }
    }
    
    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let event_bus = EventBus::default();
        let mut subscriber = event_bus.subscribe();
        
        // Create and publish an event
        let order = create_test_order();
        let event = MatchingEngineEvent::OrderAdded {
            order: order.clone(),
            timestamp: Utc::now(),
        };
        
        event_bus.publish(event.clone()).unwrap();
        
        // Receive the event
        let received = subscriber.recv().await.unwrap();
        
        match received {
            MatchingEngineEvent::OrderAdded { order: received_order, .. } => {
                assert_eq!(received_order.id, order.id);
            }
            _ => panic!("Received unexpected event type"),
        }
    }
    
    #[tokio::test]
    async fn test_event_logger() {
        let event_bus = EventBus::default();
        let event_logger = Arc::new(EventLogger::new(10));
        
        let dispatcher = EventDispatcher::new(event_bus.clone());
        dispatcher.register_handler(event_logger.clone()).await;
        
        let _handle = dispatcher.start().await;
        
        // Publish an event
        let order = create_test_order();
        let event = MatchingEngineEvent::OrderAdded {
            order,
            timestamp: Utc::now(),
        };
        
        event_bus.publish(event.clone()).unwrap();
        
        // Allow time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Check that the event was logged
        let history = event_logger.get_history().await;
        assert_eq!(history.len(), 1);
        
        match &history[0] {
            MatchingEngineEvent::OrderAdded { .. } => {
                // Test passes
            }
            _ => panic!("Logged unexpected event type"),
        }
    }
    
    #[tokio::test]
    async fn test_event_dispatcher_multiple_handlers() {
        let event_bus = EventBus::default();
        
        // Create two loggers
        let logger1 = Arc::new(EventLogger::new(10));
        let logger2 = Arc::new(EventLogger::new(10));
        
        let dispatcher = EventDispatcher::new(event_bus.clone());
        dispatcher.register_handler(logger1.clone()).await;
        dispatcher.register_handler(logger2.clone()).await;
        
        let _handle = dispatcher.start().await;
        
        // Publish events
        let order = create_test_order();
        let order_event = MatchingEngineEvent::OrderAdded {
            order,
            timestamp: Utc::now(),
        };
        
        let trade = create_test_trade();
        let trade_event = MatchingEngineEvent::TradeExecuted {
            trade,
            timestamp: Utc::now(),
        };
        
        event_bus.publish(order_event.clone()).unwrap();
        event_bus.publish(trade_event.clone()).unwrap();
        
        // Allow time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Check both loggers received the events
        let history1 = logger1.get_history().await;
        let history2 = logger2.get_history().await;
        
        assert_eq!(history1.len(), 2);
        assert_eq!(history2.len(), 2);
    }
    
    #[tokio::test]
    async fn test_persistence_handler() {
        use super::*;
        use tokio::fs;
        
        // Create a temporary directory for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        
        // Create the event bus and persistence handler
        let event_bus = EventBus::default();
        let persistence_handler = Arc::new(PersistenceEventHandler::new(&temp_path, 10).unwrap());
        
        // Create the dispatcher and register the handler
        let dispatcher = EventDispatcher::new(event_bus.clone());
        dispatcher.register_handler(persistence_handler.clone()).await;
        let _handle = dispatcher.start().await;
        
        // Create and publish an event
        let trade = create_test_trade();
        let event = MatchingEngineEvent::TradeExecuted {
            trade,
            timestamp: Utc::now(),
        };
        
        event_bus.publish(event).unwrap();
        
        // Allow time for the event to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Check if a file was created
        let mut found_file = false;
        let mut entries = fs::read_dir(&temp_path).await.unwrap();
        
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let path = entry.path();
            if path.extension().unwrap_or_default() == "jsonl" {
                found_file = true;
                
                // Read the file and verify it contains the event
                let contents = fs::read_to_string(&path).await.unwrap();
                assert!(contents.contains("TradeExecuted"));
                
                break;
            }
        }
        
        assert!(found_file, "No event file was created");
        
        // Clean up
        temp_dir.close().unwrap();
    }
} 
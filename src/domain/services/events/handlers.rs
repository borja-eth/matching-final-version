//--------------------------------------------------------------------------------------------------
// STRUCTS & TRAITS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | EventHandler            | Trait for event handling                         | handle_event      |
// | EventLogger             | Simple logging handler for events                | get_history       |
// | PersistenceEventHandler | Handler for persisting events to storage         | write_event       |
//--------------------------------------------------------------------------------------------------

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use chrono::Utc;
use tokio::sync::{Mutex, RwLock};
use tokio::io::AsyncWriteExt;
use tracing::{debug, error};
use std::sync::Arc;

use super::event_types::{MatchingEngineEvent, EventResult, EventError};

/// Event handler trait for processing events
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    /// Returns the types of events this handler processes
    fn event_types(&self) -> Vec<&'static str>;
    
    /// Processes an event
    async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()>;
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
    current_file: Mutex<Option<tokio::fs::File>>,
    /// Maximum events per file before rotation
    max_events_per_file: usize,
    /// Current event count in the current file
    event_count: AtomicUsize,
}

impl PersistenceEventHandler {
    /// Creates a new persistence handler
    pub fn new<P: AsRef<Path>>(output_dir: P, max_events_per_file: usize) -> std::io::Result<Self> {
        let path = output_dir.as_ref().to_path_buf();
        
        // Ensure directory exists
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        
        Ok(Self {
            output_dir: path,
            current_file: Mutex::new(None),
            max_events_per_file,
            event_count: AtomicUsize::new(0),
        })
    }
    
    /// Opens a new file for writing events
    async fn open_new_file(&self) -> std::io::Result<tokio::fs::File> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let filename = format!("events_{}.jsonl", timestamp);
        let path = self.output_dir.join(filename);
        
        debug!("Opening new event file: {:?}", path);
        
        let file = tokio::fs::File::create(path).await?;
        self.event_count.store(0, Ordering::SeqCst);
        
        Ok(file)
    }
    
    /// Writes an event to the current file
    async fn write_event(&self, event: &MatchingEngineEvent) -> std::io::Result<()> {
        let mut file_guard = self.current_file.lock().await;
        
        // Create file if it doesn't exist or needs rotation
        if file_guard.is_none() || self.event_count.load(Ordering::SeqCst) >= self.max_events_per_file {
            *file_guard = Some(self.open_new_file().await?);
        }
        
        // Get the file
        let file = file_guard.as_mut().unwrap();
        
        // Serialize event to JSON
        let json = serde_json::to_string(&event)?;
        
        // Write to file with newline
        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        
        // Increment event count
        self.event_count.fetch_add(1, Ordering::SeqCst);
        
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
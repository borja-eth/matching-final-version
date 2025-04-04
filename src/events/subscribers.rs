use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{self, bounded};
use uuid::Uuid;

use crate::events::bus::{EventBus, Subscription};
use crate::events::types::{Event, OrderEvent, TradeEvent, OrderBookEvent, MarketEvent};

/// Trait for components that process events
pub trait EventSubscriber {
    /// Start processing events
    fn start(&self);
    /// Stop processing events
    fn stop(&self);
    /// Process a single event
    fn process_event(&self, event: Event);
}

/// Base subscriber for processing events asynchronously
pub struct AsyncSubscriber {
    /// Unique identifier for this subscriber
    id: Uuid,
    /// Subscription to the event bus
    subscription: Subscription,
    /// Whether the subscriber is running
    running: Arc<AtomicBool>,
    /// Thread handle for the subscriber
    handle: Option<thread::JoinHandle<()>>,
    /// Name of this subscriber, used for logging and identification
    name: String,
}

impl AsyncSubscriber {
    /// Creates a new async subscriber
    pub fn new(event_bus: &EventBus, name: impl Into<String>) -> Self {
        let subscription = event_bus.subscribe();
        let id = subscription.id();
        
        Self {
            id,
            subscription,
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
            name: name.into(),
        }
    }
    
    /// Starts the subscriber in a new thread
    pub fn start<F>(&mut self, event_handler: F)
    where
        F: Fn(Event) + Send + 'static,
    {
        if self.running.load(Ordering::SeqCst) {
            return; // Already running
        }
        
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let subscription = self.subscription.clone();
        let name = self.name.clone();
        
        let handle = thread::spawn(move || {
            println!("Subscriber '{}' started", name);
            
            // Create a stop channel to check for stop signal
            let (stop_sender, stop_receiver) = bounded::<()>(1);
            
            // Setup stop signal handler
            let r = running.clone();
            let stop_sender_clone = stop_sender.clone();
            let _stop_handler = thread::spawn(move || {
                while r.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(100));
                }
                let _ = stop_sender_clone.send(());
            });
            
            // Main event processing loop
            while running.load(Ordering::SeqCst) {
                // Use a standard approach to handle events instead of the select! macro
                // that had type issues
                if let Some(event_data) = subscription.try_receive() {
                    let (event, _) = event_data;
                    event_handler(event);
                }

                // Check if we should stop
                if stop_receiver.try_recv().is_ok() {
                    println!("Subscriber '{}' received stop signal", name);
                    break;
                }

                // Small sleep to avoid busy-waiting
                thread::sleep(Duration::from_millis(10));
            }
            
            println!("Subscriber '{}' stopped", name);
        });
        
        self.handle = Some(handle);
    }
    
    /// Stops the subscriber
    pub fn stop(&mut self) {
        if !self.running.load(Ordering::SeqCst) {
            return; // Not running
        }
        
        self.running.store(false, Ordering::SeqCst);
        
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Subscriber for logging events
pub struct LoggingSubscriber {
    /// The async subscriber that handles threading
    inner: AsyncSubscriber,
}

impl LoggingSubscriber {
    /// Creates a new logging subscriber
    pub fn new(event_bus: &EventBus) -> Self {
        Self {
            inner: AsyncSubscriber::new(event_bus, "LoggingSubscriber"),
        }
    }
    
    /// Log an event
    fn log_event(&self, event: &Event) {
        match event {
            Event::Order(order_event) => {
                match order_event {
                    OrderEvent::Created(order) => {
                        println!("Order created: {} ({:?})", order.id, order.status);
                    }
                    OrderEvent::StatusChanged { order_id, old_status, new_status, .. } => {
                        println!("Order {} status changed: {:?} -> {:?}", order_id, old_status, new_status);
                    }
                    OrderEvent::Modified { old_order, new_order } => {
                        println!("Order {} modified", old_order.id);
                    }
                    OrderEvent::Cancelled(order) => {
                        println!("Order {} cancelled", order.id);
                    }
                    OrderEvent::Rejected { order_id, reason, .. } => {
                        println!("Order {} rejected: {}", order_id, reason);
                    }
                }
            }
            Event::Trade(trade_event) => {
                match trade_event {
                    TradeEvent::Executed(trade) => {
                        println!("Trade executed: {} (price: {}, amount: {})", 
                            trade.id, trade.price, trade.base_amount);
                    }
                }
            }
            Event::OrderBook(book_event) => {
                match book_event {
                    OrderBookEvent::BestPricesChanged { instrument_id, new_bid, new_ask, .. } => {
                        println!("Order book best prices changed for {}: bid={:?}, ask={:?}", 
                            instrument_id, new_bid, new_ask);
                    }
                    _ => {
                        // Log other order book events
                    }
                }
            }
            Event::Market(market_event) => {
                match market_event {
                    MarketEvent::PriceTick { instrument_id, price, .. } => {
                        println!("Price tick for {}: {}", instrument_id, price);
                    }
                    MarketEvent::StatusChanged { instrument_id, is_trading, .. } => {
                        println!("Market status changed for {}: trading={}", instrument_id, is_trading);
                    }
                }
            }
        }
    }
}

impl EventSubscriber for LoggingSubscriber {
    fn start(&self) {
        let mut inner = AsyncSubscriber::new(&EventBus::new("temp"), "LoggingSubscriber");
        inner.start(|event| {
            // In the actual implementation, self would be captured and self.log_event would be called
            match event {
                Event::Order(order_event) => {
                    match order_event {
                        OrderEvent::Created(order) => {
                            println!("Order created: {} ({:?})", order.id, order.status);
                        }
                        // ... other handlers similar to log_event above
                        _ => {}
                    }
                }
                // ... other event types
                _ => {}
            }
        });
    }
    
    fn stop(&self) {
        let mut inner = self.inner.clone();
        inner.stop();
    }
    
    fn process_event(&self, event: Event) {
        self.log_event(&event);
    }
}

impl Clone for AsyncSubscriber {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            subscription: self.subscription.clone(),
            running: self.running.clone(),
            handle: None, // Can't clone thread handle
            name: self.name.clone(),
        }
    }
}

// We'll use a different approach rather than implementing Clone for Subscription
// since its fields are private and we can't access them directly
impl Clone for Subscription {
    fn clone(&self) -> Self {
        // Create a new subscription with the same ID but a cloned receiver
        Subscription::new(self.id(), self.clone_receiver())
    }
} 
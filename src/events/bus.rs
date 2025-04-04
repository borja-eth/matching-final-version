use std::sync::Arc;
use crossbeam_channel::{Sender, Receiver, unbounded};
use parking_lot::RwLock;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::Utc;

use super::types::{Event, EventMetadata};

/// Subscription to the event bus
#[derive(Debug)]
pub struct Subscription {
    id: Uuid,
    receiver: Receiver<(Event, EventMetadata)>,
}

impl Subscription {
    /// Creates a new subscription with the given ID and receiver
    pub fn new(id: Uuid, receiver: Receiver<(Event, EventMetadata)>) -> Self {
        Self { id, receiver }
    }

    /// Returns the subscription ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Receives the next event from the subscription
    pub fn receive(&self) -> Option<(Event, EventMetadata)> {
        self.receiver.recv().ok()
    }

    /// Try to receive an event without blocking
    pub fn try_receive(&self) -> Option<(Event, EventMetadata)> {
        self.receiver.try_recv().ok()
    }

    /// Clone the receiver for this subscription - for internal use
    pub(crate) fn clone_receiver(&self) -> Receiver<(Event, EventMetadata)> {
        self.receiver.clone()
    }
}

/// High-performance event bus for distributing events to subscribers
#[derive(Debug, Clone)]
pub struct EventBus {
    // Shared state between all clones of the EventBus
    inner: Arc<RwLock<EventBusInner>>,
    // Sequence counter for event ordering
    sequence_counter: Arc<AtomicU64>,
    // Source identifier for this bus instance
    source: String,
}

/// Inner state of the event bus
#[derive(Debug)]
struct EventBusInner {
    // Map of subscription ID to sender
    senders: HashMap<Uuid, Sender<(Event, EventMetadata)>>,
}

impl EventBus {
    /// Creates a new event bus
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(EventBusInner {
                senders: HashMap::new(),
            })),
            sequence_counter: Arc::new(AtomicU64::new(1)),
            source: source.into(),
        }
    }

    /// Subscribes to events
    pub fn subscribe(&self) -> Subscription {
        let subscription_id = Uuid::new_v4();
        let (sender, receiver) = unbounded();
        
        let mut inner = self.inner.write();
        inner.senders.insert(subscription_id, sender);
        
        Subscription::new(subscription_id, receiver)
    }

    /// Unsubscribes from events
    pub fn unsubscribe(&self, subscription_id: Uuid) -> bool {
        let mut inner = self.inner.write();
        inner.senders.remove(&subscription_id).is_some()
    }

    /// Publishes an event to all subscribers
    pub fn publish(&self, event: Event) {
        // Create metadata
        let metadata = EventMetadata {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            sequence: self.sequence_counter.fetch_add(1, Ordering::SeqCst),
            source: self.source.clone(),
        };

        // Avoid holding the lock while sending to potentially slow subscribers
        let senders = {
            let inner = self.inner.read();
            inner.senders.values().cloned().collect::<Vec<_>>()
        };

        // Send to all subscribers
        for sender in senders {
            // Use try_send to avoid blocking if a subscriber's queue is full
            let _ = sender.try_send((event.clone(), metadata.clone()));
        }
    }

    /// Returns the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        let inner = self.inner.read();
        inner.senders.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OrderStatus;
    use crate::events::types::OrderEvent;
    use uuid::Uuid;

    #[test]
    fn test_subscribe_unsubscribe() {
        let bus = EventBus::new("test");
        assert_eq!(bus.subscriber_count(), 0);
        
        let subscription = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);
        
        assert!(bus.unsubscribe(subscription.id()));
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_publish_receive() {
        let bus = EventBus::new("test");
        let subscription = bus.subscribe();
        
        // Create dummy event
        let order_id = Uuid::new_v4();
        let instrument_id = Uuid::new_v4();
        let event = Event::Order(OrderEvent::StatusChanged {
            order_id,
            instrument_id,
            old_status: OrderStatus::New,
            new_status: OrderStatus::Filled,
        });
        
        // Publish event
        bus.publish(event.clone());
        
        // Receive event
        let received = subscription.receive().expect("Should receive event");
        
        // Check event content
        match &received.0 {
            Event::Order(OrderEvent::StatusChanged { order_id: rec_order_id, .. }) => {
                assert_eq!(rec_order_id, &order_id);
            }
            _ => panic!("Wrong event type received"),
        }
        
        // Check metadata
        assert_eq!(received.1.sequence, 1);
        assert_eq!(received.1.source, "test");
    }
    
    #[test]
    fn test_multiple_subscribers() {
        let bus = EventBus::new("test");
        let sub1 = bus.subscribe();
        let sub2 = bus.subscribe();
        
        // Create and publish event
        let order_id = Uuid::new_v4();
        let instrument_id = Uuid::new_v4();
        let event = Event::Order(OrderEvent::StatusChanged {
            order_id,
            instrument_id,
            old_status: OrderStatus::New,
            new_status: OrderStatus::Filled,
        });
        
        bus.publish(event);
        
        // Both subscribers should receive the event
        let _ = sub1.receive().expect("Sub1 should receive event");
        let _ = sub2.receive().expect("Sub2 should receive event");
    }
} 
//! Functions for handling domain events and publishing them to external systems.

use std::sync::Arc;

use rabbitmq::{Message, Publisher, PublisherContext};
use uuid::Uuid;
use tracing::{info, error};

use super::events::order::ResultEvent;

/// Handles a batch of domain events by publishing them to the messaging system.
///
/// # Arguments
///
/// * `events` - The vector of domain events to publish
/// * `rabbit_publisher` - The event publisher implementation
pub fn handle_event(events: Vec<ResultEvent>, rabbit_publisher: Arc<Publisher>) {
    info!("Sending event: {:?}", events);
    let event_bytes = serde_json::to_vec(&events).unwrap();
    let ctx = PublisherContext::new(&Uuid::new_v4().to_string(), None);
    let message = Message::new(event_bytes, None);
    if let Err(e) = rabbit_publisher.publish(message, ctx) {
        error!("Error publishing event: {}", e);
    }
} 
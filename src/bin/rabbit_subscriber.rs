use rabbitmq::{Message, PublisherContext, PublisherMode, RabbitMQBuilder, RabbitMQError, SubscriberMode};
use std::env;
use tokio::time::Duration;
use tracing::{info, error};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Hash)]
enum AppSubscriptions {
    Orders,
}

impl From<&'static str> for AppSubscriptions {
    fn from(s: &'static str) -> Self {
        match s {
            "Orders" => Self::Orders,
            _ => panic!("Unknown subscription: {}", s),
        }
    }
}

impl From<AppSubscriptions> for &'static str {
    fn from(queue: AppSubscriptions) -> Self {
        match queue {
            AppSubscriptions::Orders => "Orders",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum AppPublishers {
    OrderAcks,
}

impl From<&'static str> for AppPublishers {
    fn from(s: &'static str) -> Self {
        match s {
            "OrderAcks" => Self::OrderAcks,
            _ => panic!("Unknown publisher: {}", s),
        }
    }
}

impl From<AppPublishers> for &'static str {
    fn from(queue: AppPublishers) -> Self {
        match queue {
            AppPublishers::OrderAcks => "OrderAcks",
        }
    }
}

// Define a simple order acknowledgment structure
#[derive(Serialize, Deserialize)]
struct OrderAck {
    order_id: String,
    status: String, 
    timestamp: u64,
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<RabbitMQError>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get connection string from environment or use default
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    info!("Connecting to RabbitMQ at: {}", connection_string);

    // Create a builder with both subscriber and publisher
    let builder = RabbitMQBuilder::new(&connection_string, "MATCHING_ENGINE_SUBSCRIBER")
        .subscriber(AppSubscriptions::Orders, SubscriberMode::PubSub)
        .publisher(AppPublishers::OrderAcks, PublisherMode::PubSub);

    // Build both client and server
    let (client, server) = match builder.build().await {
        Ok((client, server)) => {
            info!("Successfully connected to RabbitMQ");
            (client, server)
        },
        Err(err) => {
            error!("Failed to connect to RabbitMQ: {}", err);
            return Err(Box::new(err));
        }
    };

    // Get the publishers and subscribers
    let mut publishers = client.get_publishers();
    let mut subscribers = server.get_subscribers();

    // Get the publishers and subscribers
    let order_ack_publisher = match publishers.take_ownership((AppPublishers::OrderAcks, PublisherMode::PubSub)) {
        Ok(publisher) => {
            info!("Successfully created publisher for order acknowledgments");
            publisher
        },
        Err(err) => {
            error!("Failed to create publisher for order acknowledgments: {}", err);
            return Err(Box::new(err));
        }
    };

    // Get the Orders subscriber
    let mut orders_subscriber = match subscribers.take_ownership((AppSubscriptions::Orders, SubscriberMode::PubSub)) {
        Ok(subscriber) => {
            info!("Successfully subscribed to {} queue", subscriber.queue_name());
            subscriber
        },
        Err(err) => {
            error!("Failed to subscribe to Orders queue: {}", err);
            return Err(Box::new(err));
        }
    };

    info!("Waiting for messages on Orders queue...");

    // Continuously receive messages
    loop {
        if let Some(message) = orders_subscriber.receive().await {
            if let Some(content) = &message.content {
                let content_str = String::from_utf8_lossy(content);
                info!("Received message: {}", content_str);
                
                // Process message here
                let order_id = process_message(content).await;
                
                // Send acknowledgment back to the sender
                match send_order_acknowledgment(&order_ack_publisher, &order_id).await {
                    Ok(_) => {},
                    Err(err) => {
                        // Convert other error types to RabbitMQError
                        error!("Failed to send order acknowledgment: {}", err);
                        // Continue processing even if acknowledgment fails
                    }
                }
                
                // Acknowledge the message
                if let Err(err) = orders_subscriber.ack(&message).await {
                    error!("Failed to acknowledge message: {}", err);
                    // We don't want to exit the loop on ack errors, just log and continue
                }
            } else {
                info!("Received empty message");
            }
        }
        
        // Small delay to prevent tight loop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // This line will never be reached due to the infinite loop above,
    // but it satisfies the compiler's type checking for the function return type
    #[allow(unreachable_code)]
    Ok(())
}

/// Process received message and return the order ID
async fn process_message(content: &[u8]) -> String {
    // In a real implementation, you would:
    // 1. Deserialize the order from JSON/binary format
    // 2. Validate the order
    // 3. Add it to the order book or execute it
    // 4. Return the order ID for acknowledgment

    // For this example, we'll just simulate processing
    info!("Processing message of {} bytes", content.len());
    
    // For demo, generate a random order ID (in production, extract from the message)
    let order_id = Uuid::new_v4().to_string();
    
    // Simulate some processing time
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    order_id
}

/// Send an acknowledgment for a processed order
/// This now returns the same error type as other functions
async fn send_order_acknowledgment(publisher: &rabbitmq::Publisher, order_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create an acknowledgment message
    let ack = OrderAck {
        order_id: order_id.to_string(),
        status: "ACCEPTED".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        message: "Order has been received and is being processed".to_string(),
    };
    
    // Serialize the ack to JSON
    let ack_json = match serde_json::to_string(&ack) {
        Ok(json) => json,
        Err(err) => {
            return Err(Box::new(err));
        }
    };
    
    info!("Sending acknowledgment for order {}: {}", order_id, ack_json);
    
    // Send the acknowledgment
    match publisher.publish(
        Message::from(ack_json.as_bytes()),
        PublisherContext::new("order_ack_req", Some(order_id.to_string())),
    ) {
        Ok(_) => Ok(()),
        Err(err) => Err(Box::new(err)),
    }
} 
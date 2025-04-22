mod domain;

use chrono::Utc;
use uuid::Uuid;
use tracing_subscriber;
use rabbitmq::{Message, PublisherContext, PublisherMode, RabbitMQBuilder, RabbitMQError, SubscriberMode};
use std::env;
use tokio::time::Duration;

use ultimate_matching::{
    OrderType, Order, Side, OrderStatus, TimeInForce, MatchingEngine,
    domain::services::events::{EventBus, EventDispatcher, EventHandler, MatchingEngineEvent, EventResult, PersistenceEventHandler},
    domain::models::types::CreatedFrom
};

/// Simple event handler that prints events to the console
struct ConsoleEventHandler;

#[async_trait::async_trait]
impl EventHandler for ConsoleEventHandler {
    fn event_types(&self) -> Vec<&'static str> {
        vec![
            "OrderAdded", 
            "OrderMatched", 
            "OrderCancelled", 
            "TradeExecuted",
            "DepthUpdated"
        ]
    }
    
    async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()> {
        match &event {
            MatchingEngineEvent::OrderAdded { order, timestamp } => {
                println!("[{}] Order added: {} {} at {:?}", 
                    timestamp,
                    match order.side {
                        Side::Bid => "BUY",
                        Side::Ask => "SELL"
                    },
                    order.base_amount,
                    order.limit_price
                );
            },
            MatchingEngineEvent::TradeExecuted { trade, timestamp } => {
                println!("[{}] Trade executed: {} @ {}", 
                    timestamp,
                    trade.base_amount,
                    trade.price
                );
            },
            _ => {} // Ignore other events
        }
        Ok(())
    }
}

/// Create a test order
fn create_test_order(side: Side, price: f64, qty: f64, instrument_id: Uuid) -> Order {
    let now = Utc::now();
    
    // Convert to integer values with 6 decimal places of precision
    let price_i64 = (price * 1_000_000.0) as i64;
    let qty_u64 = (qty * 1_000_000.0) as u64;
    let quote_amount = ((price * qty) * 1_000_000.0) as u64;
    
    Order {
        id: Uuid::new_v4(),
        ext_id: Some("example-order".to_string()),
        account_id: Uuid::new_v4(),
        order_type: OrderType::Limit,
        instrument_id,
        side,
        limit_price: Some(price_i64),
        trigger_price: None,
        base_amount: qty_u64,
        remaining_base: qty_u64,
        filled_quote: 0,
        filled_base: 0,
        remaining_quote: quote_amount,
        expiration_date: now + chrono::Duration::days(7),
        status: OrderStatus::Submitted,
        created_at: now,
        updated_at: now,
        trigger_by: None,
        created_from: CreatedFrom::Api,
        sequence_id: 0,
        time_in_force: TimeInForce::GTC,
    }
}

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

#[tokio::main]
async fn main() -> Result<(), Box<RabbitMQError>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get connection string from environment or use default
    let connection_string = env::var("RABBITMQ_URL")
        .unwrap_or_else(|_| "amqp://guest:guest@localhost:5672".to_string());

    println!("Connecting to RabbitMQ at: {}", connection_string);

    // Create a builder with a subscriber
    let builder = RabbitMQBuilder::new(&connection_string, "SUBSCRIBER_APP")
        .subscriber(AppSubscriptions::Orders, SubscriberMode::PubSub);

    // Build the server (subscriber)
    let server = match builder.build().await {
        Ok(server) => {
            println!("Successfully connected to RabbitMQ");
            server
        },
        Err(err) => {
            eprintln!("Failed to connect to RabbitMQ: {}", err);
            return Err(Box::new(err));
        }
    };

    // Get the subscribers
    let mut subscribers = server.get_subscribers();

    // Get the Orders subscriber
    let mut orders_subscriber = match subscribers.take_ownership((AppSubscriptions::Orders, SubscriberMode::PubSub)) {
        Ok(subscriber) => {
            println!("Successfully subscribed to Orders queue");
            subscriber
        },
        Err(err) => {
            eprintln!("Failed to subscribe to Orders queue: {}", err);
            return Err(Box::new(err));
        }
    };

    println!("Waiting for messages on Orders queue...");

    // Continuously receive messages
    loop {
        if let Some(message) = orders_subscriber.receive().await {
            println!("Received message from queue: {}", orders_subscriber.queue_name());
            
            if let Some(content) = &message.content {
                let content_str = String::from_utf8_lossy(content);
                println!("Message content: {}", content_str);
                
                // Handle message logic here
                
                // Acknowledge the message
                if let Err(err) = orders_subscriber.ack(&message).await {
                    eprintln!("Failed to acknowledge message: {}", err);
                    // We don't want to exit the loop on ack errors, just log and continue
                }
            } else {
                println!("Received empty message");
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

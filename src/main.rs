use std::sync::Arc;
use std::process;

use dotenv::dotenv;
use rabbitmq::{PublisherMode, RabbitMQBuilder, SubscriberMode};
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{info, error};

use ultimate_matching::{
    Config,
    OrderbookManagerServiceImpl,
    domain::services::event_manager::{
        EventManagerService,
        event_manager_service::EventManagerServiceImpl
    },
    outbounds::events::{
        market::MarketEventHandler,
        order::OrderEventHandler
    },
    inbounds::handlers::{
        cancel_handler::handle_cancel_request,
        place_handler::handle_place_request,
        snapshot_handler::handle_snapshot_request,
        trading_status_handler::handle_trading_status_request
    }
};

/// RabbitMQ subscription types for the matching engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MatchingEngineSubscriptions {
    /// New order placements
    PlaceOrder,
    /// Order cancellations
    CancelOrder,
    /// Orderbook snapshot requests
    SnapshotRequest,
    /// Trading status requests
    TradingStatus,
}

impl From<&'static str> for MatchingEngineSubscriptions {
    fn from(s: &'static str) -> Self {
        match s {
            "PlaceOrder" => Self::PlaceOrder,
            "CancelOrder" => Self::CancelOrder,
            "SnapshotRequest" => Self::SnapshotRequest,
            "TradingStatus" => Self::TradingStatus,
            _ => panic!("Unknown subscription type: {}", s),
        }
    }
}

impl From<MatchingEngineSubscriptions> for &'static str {
    fn from(subscription: MatchingEngineSubscriptions) -> Self {
        match subscription {
            MatchingEngineSubscriptions::PlaceOrder => "PlaceOrder",
            MatchingEngineSubscriptions::CancelOrder => "CancelOrder", 
            MatchingEngineSubscriptions::SnapshotRequest => "SnapshotRequest",
            MatchingEngineSubscriptions::TradingStatus => "TradingStatus",
        }
    }
}

/// Initialize tracing for the application
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
}

/// Get configuration with Docker support
fn get_config() -> Config {
    match Config::from_env() {
        config => {
            // Check if we're in a Docker environment and adjust RabbitMQ URL if needed
            let mut config = config;
            let is_docker_env = true; // Set this based on your deployment environment
            
            if is_docker_env {
                info!("Running in Docker environment, using container IP");
                config.rabbit_url = "amqp://guest:guest@172.17.0.2:5672".to_string();
            }
            
            info!("Loaded configuration with {} instruments", config.instruments.len());
            info!("Using RabbitMQ URL: {}", config.rabbit_url);
            config
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize environment variables and tracing
    dotenv().ok();
    init_tracing();
    info!("Starting ultimate matching engine...");

    // Load configuration with Docker support
    let config = get_config();

    // Create communication channel for orderbook results
    let (_result_sender, result_receiver) = mpsc::channel(100_000);

    // Initialize event manager service
    let event_manager = match tokio::time::timeout(
        Duration::from_secs(5),
        EventManagerServiceImpl::new(&config)
    ).await {
        Ok(service) => {
            info!("Event manager service initialized");
            service
        },
        Err(_) => {
            error!("Timed out connecting to RabbitMQ for event manager after 5 seconds");
            // Try direct connection to diagnose
            info!("Attempting direct TCP connection to diagnose...");
            match tokio::net::TcpStream::connect("172.17.0.2:5672").await {
                Ok(_) => info!("TCP connection succeeded, but AMQP protocol timed out"),
                Err(e) => error!("TCP connection failed: {}", e),
            };
            
            error!("Exiting due to RabbitMQ connection failure");
            process::exit(1);
        }
    };

    // Start event manager thread
    let event_manager_handle = event_manager.run(result_receiver);
    info!("Event manager thread started");

    // Initialize orderbook manager service
    let orderbook_manager = Arc::new(OrderbookManagerServiceImpl::new(config.instruments.clone()));
    info!("Orderbook manager service initialized");

    // Initialize event handlers
    let _market_event_handler = MarketEventHandler::new();
    let _order_event_handler = OrderEventHandler::new();
    info!("Event handlers initialized");

    // Connect to RabbitMQ
    info!("Connecting to RabbitMQ at: {}", config.rabbit_url);
    let builder = RabbitMQBuilder::new(&config.rabbit_url, &config.app_id)
        .subscriber(MatchingEngineSubscriptions::PlaceOrder, SubscriberMode::PubSub)
        .subscriber(MatchingEngineSubscriptions::CancelOrder, SubscriberMode::PubSub)
        .subscriber(MatchingEngineSubscriptions::SnapshotRequest, SubscriberMode::PubSub)
        .subscriber(MatchingEngineSubscriptions::TradingStatus, SubscriberMode::PubSub)
        .publisher("events", PublisherMode::Broadcast);

    // Build RabbitMQ client and server with timeout
    let connection_result = match tokio::time::timeout(
        Duration::from_secs(5), 
        builder.build()
    ).await {
        Ok(result) => result,
        Err(_) => {
            error!("Connection to RabbitMQ timed out after 5 seconds");
            // Try direct connection to diagnose
            info!("Attempting direct TCP connection to diagnose...");
            match tokio::net::TcpStream::connect("172.17.0.2:5672").await {
                Ok(_) => info!("TCP connection succeeded, but AMQP protocol timed out"),
                Err(e) => error!("TCP connection failed: {}", e),
            };
            
            error!("Exiting due to RabbitMQ connection timeout");
            process::exit(1);
        }
    };
    
    let (client, server) = match connection_result {
        Ok((client, server)) => {
            info!("Successfully connected to RabbitMQ");
            (client, server)
        },
        Err(err) => {
            error!("Failed to connect to RabbitMQ: {}", err);
            process::exit(1);
        }
    };

    let mut subscribers = server.get_subscribers();
    
    // Get the event publisher
    let mut publishers = client.get_publishers();
    let _event_publisher = match publishers.take_ownership(("events", PublisherMode::Broadcast)) {
        Ok(publisher) => {
            info!("Created events publisher");
            publisher
        },
        Err(err) => {
            error!("Failed to create events publisher: {}", err);
            process::exit(1);
        }
    };

    // Get each subscriber
    let mut place_order_subscriber = match subscribers.take_ownership((MatchingEngineSubscriptions::PlaceOrder, SubscriberMode::PubSub)) {
        Ok(subscriber) => {
            info!("Subscribed to PlaceOrder queue: {}", subscriber.queue_name());
            subscriber
        },
        Err(err) => {
            error!("Failed to subscribe to PlaceOrder queue: {}", err);
            process::exit(1);
        }
    };

    let mut cancel_order_subscriber = match subscribers.take_ownership((MatchingEngineSubscriptions::CancelOrder, SubscriberMode::PubSub)) {
        Ok(subscriber) => {
            info!("Subscribed to CancelOrder queue: {}", subscriber.queue_name());
            subscriber
        },
        Err(err) => {
            error!("Failed to subscribe to CancelOrder queue: {}", err);
            process::exit(1);
        }
    };

    let mut snapshot_subscriber = match subscribers.take_ownership((MatchingEngineSubscriptions::SnapshotRequest, SubscriberMode::PubSub)) {
        Ok(subscriber) => {
            info!("Subscribed to SnapshotRequest queue: {}", subscriber.queue_name());
            subscriber
        },
        Err(err) => {
            error!("Failed to subscribe to SnapshotRequest queue: {}", err);
            process::exit(1);
        }
    };

    let mut trading_status_subscriber = match subscribers.take_ownership((MatchingEngineSubscriptions::TradingStatus, SubscriberMode::PubSub)) {
        Ok(subscriber) => {
            info!("Subscribed to TradingStatus queue: {}", subscriber.queue_name());
            subscriber
        },
        Err(err) => {
            error!("Failed to subscribe to TradingStatus queue: {}", err);
            process::exit(1);
        }
    };

    info!("Matching engine ready to process orders");

    // Process incoming messages
    let orderbook_manager_clone = orderbook_manager.clone();
    let place_order_task = tokio::spawn(async move {
        loop {
            if let Some(message) = place_order_subscriber.receive().await {
                if let Some(content) = &message.content {
                    info!("Received place order request");
                    match handle_place_request(content.to_vec(), orderbook_manager_clone.clone()) {
                        Ok(_) => info!("Successfully processed place order request"),
                        Err(e) => error!("Error processing place order request: {}", e),
                    }
                    
                    if let Err(e) = place_order_subscriber.ack(&message).await {
                        error!("Failed to acknowledge place order message: {}", e);
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    let orderbook_manager_clone = orderbook_manager.clone();
    let cancel_order_task = tokio::spawn(async move {
        loop {
            if let Some(message) = cancel_order_subscriber.receive().await {
                if let Some(content) = &message.content {
                    info!("Received cancel order request");
                    match handle_cancel_request(content.to_vec(), orderbook_manager_clone.clone()) {
                        Ok(_) => info!("Successfully processed cancel order request"),
                        Err(e) => error!("Error processing cancel order request: {}", e),
                    }
                    
                    if let Err(e) = cancel_order_subscriber.ack(&message).await {
                        error!("Failed to acknowledge cancel order message: {}", e);
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    let orderbook_manager_clone = orderbook_manager.clone();
    let snapshot_task = tokio::spawn(async move {
        loop {
            if let Some(message) = snapshot_subscriber.receive().await {
                if let Some(content) = &message.content {
                    info!("Received snapshot request");
                    match handle_snapshot_request(content.to_vec(), orderbook_manager_clone.clone()) {
                        Ok(_) => info!("Successfully processed snapshot request"),
                        Err(e) => error!("Error processing snapshot request: {}", e),
                    }
                    
                    if let Err(e) = snapshot_subscriber.ack(&message).await {
                        error!("Failed to acknowledge snapshot message: {}", e);
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    let orderbook_manager_clone = orderbook_manager.clone();
    let trading_status_task = tokio::spawn(async move {
        loop {
            if let Some(message) = trading_status_subscriber.receive().await {
                if let Some(content) = &message.content {
                    info!("Received trading status request");
                    match handle_trading_status_request(content.to_vec(), orderbook_manager_clone.clone()) {
                        Ok(_) => info!("Successfully processed trading status request"),
                        Err(e) => error!("Error processing trading status request: {}", e),
                    }
                    
                    if let Err(e) = trading_status_subscriber.ack(&message).await {
                        error!("Failed to acknowledge trading status message: {}", e);
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    // Start all tasks
    match tokio::try_join!(
        place_order_task,
        cancel_order_task,
        snapshot_task,
        trading_status_task
    ) {
        Ok(_) => {},
        Err(e) => {
            error!("Error in task: {}", e);
            // In production, would implement proper error handling and recovery
        }
    }

    // Wait for event manager thread to complete
    if let Err(e) = event_manager_handle.join() {
        error!("Error joining event manager thread: {:?}", e);
    }

    info!("Matching engine shutting down");
}

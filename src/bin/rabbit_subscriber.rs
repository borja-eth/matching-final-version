use std::sync::Arc;
use std::net::{TcpStream, SocketAddr};
use std::time::Duration as StdDuration;

use rabbitmq::{RabbitMQBuilder, RabbitMQError, SubscriberMode};
use tokio::time::Duration;
use tracing::{info, error, warn};
use uuid::Uuid;
use std::str::FromStr;

use ultimate_matching::{
    Config,
    OrderbookManagerServiceImpl,
    domain::services::orderbook_manager::OrderbookManagerService,
    inbounds::handlers::{
        cancel_handler::handle_cancel_request,
        place_handler::handle_place_request,
        snapshot_handler::handle_snapshot_request,
        trading_status_handler::handle_trading_status_request,
    },
};

/// +----------------------------------------------------------+
/// | CONSTANTS | ENUMS | TRAITS | STRUCTS | FUNCTIONS         |
/// +----------+-------+-------+------------------------------+
/// | Constants:                                               |
/// |   - MOXOR_ORDER_PLACE_CHANNEL                           |
/// |   - ORDER_CANCEL_CHANNEL                                 |
/// |   - ORDER_SNAPSHOT_CHANNEL                               |
/// |   - ORDER_TRADING_STATUS_CHANNEL                         |
/// |                                                          |
/// | Enums:                                                   |
/// |   - MoxorAIQueues                                       |
/// |                                                          |
/// | Structs:                                                 |
/// |   - RabbitMQSubscriber                                   |
/// +----------------------------------------------------------+

/// Channel template for placing orders with instrument placeholder (*)
const MOXOR_ORDER_PLACE_CHANNEL: &str = "moxor.orders.*.place";

/// Channel template for canceling orders with instrument placeholder (*)
const ORDER_CANCEL_CHANNEL: &str = "moxor.orders.*.cancel";

/// Channel template for requesting orderbook snapshots with instrument placeholder (*)
const ORDER_SNAPSHOT_CHANNEL: &str = "moxor.prices.*.snapshot";

/// Channel template for checking trading status with instrument placeholder (*)
const ORDER_TRADING_STATUS_CHANNEL: &str = "moxor.prices.*.trading.status";

/// Queue names for the matching engine
#[derive(Debug, Hash, Eq, PartialEq)]
pub enum MoxorAIQueues {
    /// Queue for handling orders
    MoxorAI,
    
    /// Queue for handling prices
    MoxorAIPrices,
    
    /// Queue for results
    MoxorAIResults,
}

impl From<&'static str> for MoxorAIQueues {
    fn from(s: &'static str) -> Self {
        match s {
            "MoxorAI" => MoxorAIQueues::MoxorAI,
            "MoxorAIPrices" => MoxorAIQueues::MoxorAIPrices,
            "MoxorAIResults" => MoxorAIQueues::MoxorAIResults,
            _ => panic!("Invalid subscription type"),
        }
    }
}

impl From<MoxorAIQueues> for &'static str {
    fn from(s: MoxorAIQueues) -> Self {
        match s {
            MoxorAIQueues::MoxorAI => "MoxorAI",
            MoxorAIQueues::MoxorAIPrices => "MoxorAIPrices",
            MoxorAIQueues::MoxorAIResults => "MoxorAIResults",
        }
    }
}

/// RabbitMQ subscriber to handle incoming orderbook operations
pub struct RabbitMQSubscriber {
    /// RabbitMQ connection URL
    connection_url: String,
    
    /// Application ID
    app_id: String,
    
    /// Orderbook manager service
    orderbook_manager: Arc<dyn OrderbookManagerService>,
}

impl RabbitMQSubscriber {
    /// Creates a new RabbitMQ subscriber
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    /// * `orderbook_manager` - Service for managing orderbooks
    ///
    /// # Returns
    ///
    /// A new RabbitMQSubscriber instance
    pub fn new(config: &Config, orderbook_manager: Arc<dyn OrderbookManagerService>) -> Self {
        Self {
            connection_url: config.rabbit_url.clone(),
            app_id: config.app_id.clone(),
            orderbook_manager,
        }
    }

    /// Builds channel names by replacing placeholder in the template with instrument IDs
    ///
    /// # Arguments
    ///
    /// * `channel_template` - Channel template with a placeholder (*)
    /// * `instruments` - List of instrument IDs
    ///
    /// # Returns
    ///
    /// Vector of channel names with instrument IDs
    fn build_channels(&self, channel_template: &str, instruments: &[Uuid]) -> Vec<String> {
        instruments
            .iter()
            .map(|instrument| channel_template.replace('*', &instrument.to_string()))
            .collect()
    }

    /// Starts the RabbitMQ subscriber and processes incoming messages
    ///
    /// # Arguments
    ///
    /// * `instruments` - List of instruments to subscribe to
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn start(&self, instruments: &[Uuid]) -> Result<(), RabbitMQError> {
        info!("Starting RabbitMQ subscriber for {} instruments", instruments.len());
        
        // Build channel lists for each message type
        let mut order_topics = self.build_channels(ORDER_CANCEL_CHANNEL, instruments);
        order_topics.extend(self.build_channels(MOXOR_ORDER_PLACE_CHANNEL, instruments));
        
        let snapshots_channels = self.build_channels(ORDER_SNAPSHOT_CHANNEL, instruments);
        let trading_status_channels = self.build_channels(ORDER_TRADING_STATUS_CHANNEL, instruments);
        
        info!("Connecting to RabbitMQ at: {}", self.connection_url);
        info!("Order topics: {:?}", order_topics);
        info!("Snapshot topics: {:?}", snapshots_channels);
        info!("Trading status topics: {:?}", trading_status_channels);
        
        // Perform pre-connection diagnostics
        self.diagnose_connection(&self.connection_url).await;
        
        info!("Building RabbitMQ server with a 10-second timeout...");
        
        // Try with retry and timeout
        let mut retry_count = 0;
        let max_retries = 3;
        let mut server_result = None;
        
        while retry_count < max_retries {
            info!("Connection attempt {} of {}", retry_count + 1, max_retries);
            
            // Create a new builder for each attempt
            let builder = RabbitMQBuilder::new(&self.connection_url, &self.app_id)
                .subscriber(
                    MoxorAIQueues::MoxorAI,
                    SubscriberMode::Topics {
                        topics: order_topics.clone(),
                    },
                )
                .subscriber(
                    MoxorAIQueues::MoxorAIPrices,
                    SubscriberMode::Topics {
                        topics: snapshots_channels.clone(),
                    },
                )
                .subscriber(
                    MoxorAIQueues::MoxorAIPrices,
                    SubscriberMode::Topics {
                        topics: trading_status_channels.clone(),
                    },
                );
            
            match tokio::time::timeout(
                Duration::from_secs(10),
                builder.build()
            ).await {
                Ok(result) => {
                    server_result = Some(result);
                    break;
                },
                Err(_) => {
                    error!("Connection attempt {} timed out after 10 seconds", retry_count + 1);
                    
                    if retry_count < max_retries - 1 {
                        let backoff_seconds = 2u64.pow(retry_count + 1);
                        info!("Waiting {} seconds before retry...", backoff_seconds);
                        tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
                    }
                    
                    retry_count += 1;
                }
            }
        }
        
        // If all retries failed, perform diagnostic and return error
        if server_result.is_none() {
            error!("All connection attempts failed");
            self.perform_connection_diagnostics(&self.connection_url).await;
            return Err(RabbitMQError::ConnectionError("Connection timed out after multiple attempts".to_string()));
        }
        
        let server = match server_result.unwrap() {
            Ok(s) => {
                info!("Successfully connected to RabbitMQ and built server");
                s
            },
            Err(err) => {
                error!("Failed to connect to RabbitMQ: {}", err);
                self.perform_connection_diagnostics(&self.connection_url).await;
                return Err(err);
            }
        };
        
        let mut subscribers = server.get_subscribers();

        // Process orders (place and cancel)
        let orders_manager = self.orderbook_manager.clone();
        if let Ok(mut subscriber) = subscribers.take_ownership((
            MoxorAIQueues::MoxorAI,
            SubscriberMode::Topics {
                topics: order_topics,
            },
        )) {
            info!("Successfully subscribed to order topics");
            tokio::spawn(async move {
                while let Some(msg) = subscriber.receive().await {
                    if let Some(content) = msg.content.clone() {
                        // Extract routing key from deliver info
                        let routing_key = if let Some(deliver) = &msg.deliver {
                            deliver.routing_key().to_string()
                        } else {
                            String::new()
                        };
                        
                        info!("Received order message with routing key: {}", routing_key);
                        
                        // Process the message based on routing key
                        if routing_key.contains(".cancel") {
                            match handle_cancel_request(content, orders_manager.clone()) {
                                Ok(_) => info!("Successfully processed cancel order request"),
                                Err(e) => error!("Error processing cancel order request: {}", e),
                            }
                        } else {
                            match handle_place_request(content, orders_manager.clone()) {
                                Ok(_) => info!("Successfully processed place order request"),
                                Err(e) => error!("Error processing place order request: {}", e),
                            }
                        }
                    }
                    
                    if let Err(e) = subscriber.ack(&msg).await {
                        error!("Failed to acknowledge order message: {}", e);
                    }
                    
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            });
        } else {
            warn!("Failed to subscribe to order topics, continuing with other subscribers");
        }

        // Process trading status requests
        let trading_status_manager = self.orderbook_manager.clone();
        if let Ok(mut subscriber) = subscribers.take_ownership((
            MoxorAIQueues::MoxorAIPrices,
            SubscriberMode::Topics {
                topics: trading_status_channels,
            },
        )) {
            info!("Successfully subscribed to trading status topics");
            tokio::spawn(async move {
                while let Some(msg) = subscriber.receive().await {
                    if let Some(content) = msg.content.clone() {
                        info!("Received trading status request");
                        if let Err(e) = handle_trading_status_request(content, trading_status_manager.clone()) {
                            error!("Error processing trading status request: {}", e);
                        }
                    }
                    
                    if let Err(e) = subscriber.ack(&msg).await {
                        error!("Failed to acknowledge trading status message: {}", e);
                    }
                    
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            });
        } else {
            warn!("Failed to subscribe to trading status topics, continuing with other subscribers");
        }

        // Process snapshot requests
        let snapshot_manager = self.orderbook_manager.clone();
        if let Ok(mut subscriber) = subscribers.take_ownership((
            MoxorAIQueues::MoxorAIPrices,
            SubscriberMode::Topics {
                topics: snapshots_channels,
            },
        )) {
            info!("Successfully subscribed to snapshot topics");
            tokio::spawn(async move {
                while let Some(msg) = subscriber.receive().await {
                    if let Some(content) = msg.content.clone() {
                        info!("Received snapshot request");
                        if let Err(e) = handle_snapshot_request(content, snapshot_manager.clone()) {
                            error!("Error processing snapshot request: {}", e);
                        }
                    }
                    
                    if let Err(e) = subscriber.ack(&msg).await {
                        error!("Failed to acknowledge snapshot message: {}", e);
                    }
                    
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            });
        } else {
            warn!("Failed to subscribe to snapshot topics, continuing with other subscribers");
        }

        info!("RabbitMQ subscriber started successfully");
        Ok(())
    }
    
    /// Diagnose RabbitMQ connection before attempting to connect
    async fn diagnose_connection(&self, connection_url: &str) {
        info!("Performing pre-connection diagnostics for {}", connection_url);
        
        // Parse URL to extract host and port
        if let Some((host, port)) = self.extract_host_port(connection_url) {
            info!("Extracted host: {}, port: {}", host, port);
            
            // Try TCP connection
            match TcpStream::connect_timeout(&SocketAddr::from_str(&format!("{}:{}", host, port)).unwrap_or_else(|_| {
                error!("Failed to parse address {}:{}", host, port);
                "127.0.0.1:5672".parse().unwrap()
            }), StdDuration::from_secs(2)) {
                Ok(_) => info!("TCP connection to {}:{} successful", host, port),
                Err(e) => {
                    error!("TCP connection to {}:{} failed: {}", host, port, e);
                    info!("Network status: checking DNS resolution...");
                    
                    // Try to resolve DNS just in case
                    if let Ok(addrs) = tokio::net::lookup_host(format!("{}:{}", host, port)).await {
                        let addrs: Vec<_> = addrs.collect();
                        info!("DNS resolution for {}: {:?}", host, addrs);
                    } else {
                        error!("DNS resolution for {} failed", host);
                    }
                }
            }
        } else {
            error!("Failed to extract host and port from connection URL: {}", connection_url);
        }
    }
    
    /// Perform enhanced diagnostics after connection timeout
    async fn perform_connection_diagnostics(&self, connection_url: &str) {
        error!("Performing detailed connection diagnostics after failure");
        
        // Try direct connection to diagnose
        info!("Attempting direct TCP connection to diagnose...");
        
        // Extract host and port from the URL
        if let Some((host, port)) = self.extract_host_port(connection_url) {
            info!("Attempting TCP connection to {}:{}", host, port);
            
            match tokio::net::TcpStream::connect(format!("{}:{}", host, port)).await {
                Ok(_) => {
                    info!("Direct TCP connection to {}:{} succeeded, issue is likely with AMQP protocol", host, port);
                    info!("Possible issues:");
                    info!("1. RabbitMQ service might not be running");
                    info!("2. Authentication failed (check credentials)");
                    info!("3. Virtual host might not exist or permissions are incorrect");
                },
                Err(e) => {
                    error!("Direct TCP connection to {}:{} failed: {}", host, port, e);
                    error!("Network connectivity issues:");
                    error!("1. RabbitMQ server might be down");
                    error!("2. Network route blocked (firewall?)");
                    error!("3. Incorrect host/IP or port");
                }
            }
        } else {
            error!("Could not parse RabbitMQ URL: {}", connection_url);
        }
        
        // Try default RabbitMQ docker address as backup
        info!("Trying fallback to default Docker RabbitMQ address 172.17.0.2:5672");
        match tokio::net::TcpStream::connect("172.17.0.2:5672").await {
            Ok(_) => info!("Connection to default Docker RabbitMQ address succeeded"),
            Err(e) => error!("Connection to default Docker RabbitMQ address failed: {}", e),
        }
        
        // Check local loopback as sanity check
        match tokio::net::TcpStream::connect("127.0.0.1:5672").await {
            Ok(_) => info!("Connection to localhost RabbitMQ succeeded - consider updating URL to use localhost"),
            Err(e) => error!("Connection to localhost RabbitMQ failed: {}", e),
        }
    }
    
    /// Extract host and port from an AMQP URL
    fn extract_host_port(&self, url: &str) -> Option<(String, String)> {
        if url.starts_with("amqp://") {
            let url_without_protocol = url.trim_start_matches("amqp://");
            
            // Handle credentials if present
            let url_parts = if url_without_protocol.contains('@') {
                url_without_protocol.split('@').nth(1)
            } else {
                Some(url_without_protocol)
            };
            
            if let Some(address_part) = url_parts {
                // Split host and port
                let host_port: Vec<&str> = address_part.split(':').collect();
                if host_port.len() >= 2 {
                    let host = host_port[0].to_string();
                    // Handle potential virtual host or query params
                    let port = if host_port[1].contains('/') {
                        host_port[1].split('/').next().unwrap_or("5672")
                    } else {
                        host_port[1]
                    }.to_string();
                    
                    return Some((host, port));
                }
            }
        }
        
        error!("Failed to parse AMQP URL: {}", url);
        None
    }
}

/// Initialize tracing for the application
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
}

#[tokio::main]
async fn main() -> Result<(), RabbitMQError> {
    // Initialize environment variables and tracing
    dotenv::dotenv().ok();
    init_tracing();
    
    info!("Starting RabbitMQ subscriber...");

    // Load configuration with more detailed logging
    let config = match Config::try_from_env() {
        Ok(config) => {
            info!("Loaded configuration with {} instruments", config.instruments.len());
            info!("RabbitMQ URL from config: {}", config.rabbit_url);
            
            // Check if we're in Docker and adjust if needed
            let is_docker_env = std::env::var("DOCKER_ENV").unwrap_or_else(|_| "false".to_string()) == "true";
            
            let mut adjusted_config = config;
            if is_docker_env {
                info!("Running in Docker environment, using container IP");
                adjusted_config.rabbit_url = "amqp://guest:guest@172.17.0.2:5672".to_string();
            }
            
            adjusted_config
        },
        Err(err) => {
            warn!("Failed to load config from environment: {}", err);
            info!("Using default configuration");
            let mut default_config = Config::default();
            
            // Try to determine if we're in Docker
            if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
                if hostname.trim().len() > 8 && hostname.trim().chars().all(|c| c.is_alphanumeric() || c == '-') {
                    info!("Detected Docker environment (hostname pattern)");
                    default_config.rabbit_url = "amqp://guest:guest@172.17.0.2:5672".to_string();
                }
            }
            
            default_config
        }
    };
    
    // The RabbitMQ URL from .env should now work with our properly exposed container
    info!("Using RabbitMQ URL: {}", config.rabbit_url);

    // Get orderbook manager
    let orderbook_manager = match get_orderbook_manager() {
        Ok(manager) => {
            info!("Orderbook manager service initialized");
            manager
        },
        Err(err) => {
            error!("Failed to initialize orderbook manager: {}", err);
            return Err(RabbitMQError::ConnectionError(format!("Failed to initialize orderbook manager: {}", err)));
        }
    };

    // Create and start the RabbitMQ subscriber
    let subscriber = RabbitMQSubscriber::new(&config, orderbook_manager);
    subscriber.start(&config.instruments).await?;

    // Keep the main task alive 
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Create an OrderbookManagerService instance
fn get_orderbook_manager() -> Result<Arc<dyn OrderbookManagerService>, String> {
    // Create an OrderbookManagerServiceImpl
    let config = match Config::try_from_env() {
        Ok(config) => config,
        Err(_) => {
            // Create default config with the Docker container's IP
            let mut default_config = Config::default();
            // Update the RabbitMQ URL to use the container's IP
            default_config.rabbit_url = "amqp://guest:guest@172.17.0.2:5672".to_string();
            info!("Using container IP for RabbitMQ: {}", default_config.rabbit_url);
            default_config
        }
    };
    
    Ok(Arc::new(OrderbookManagerServiceImpl::new(config.instruments.clone())))
} 
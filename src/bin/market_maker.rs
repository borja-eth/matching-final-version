//--------------------------------------------------------------------------------------------------
// HIGH-PERFORMANCE MARKET MAKER
//--------------------------------------------------------------------------------------------------
// This module implements a high-performance market maker that connects to the matching engine API
// and places orders at a high rate (up to 250K orders per second).
//
// | Component                | Description                                                |
// |--------------------------|-----------------------------------------------------------|
// | MarketMaker              | Core market maker implementation                          |
// | OrderBatch               | Batch of orders to be sent                                |
// | MarketMakerConfig        | Configuration for the market maker                        |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | MarketMaker             | Main market maker component                       | run, start        |
// | MarketMakerConfig       | Configuration parameters                          | from_file, new    |
// | OrderGenerator          | Generates orders for market making                | generate_orders   |
// | ApiClient              | Client for the matching engine API                | place_order       |
//--------------------------------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use std::net::{TcpStream, SocketAddr};
use std::str::FromStr;

use chrono::Utc;
use rabbitmq::{RabbitMQBuilder, RabbitMQError, PublisherMode};
use rand::{thread_rng, Rng};
use tokio::sync::Mutex;
use tokio::time::Duration;
use tracing::{info, error, warn};
use uuid::Uuid;

use ultimate_matching::{
    Config,
    inbounds::dtos::{PlaceOrderRequest, CancelOrderRequest},
    domain::models::types::{Side, OrderType, TimeInForce},
};

/// +----------------------------------------------------------+
/// | CONSTANTS | ENUMS | TRAITS | STRUCTS | FUNCTIONS         |
/// +----------+-------+-------+------------------------------+
/// | Constants:                                               |
/// |   - MOXOR_ORDER_PLACE_CHANNEL                           |
/// |   - ORDER_CANCEL_CHANNEL                                 |
/// |                                                          |
/// | Enums:                                                   |
/// |   - MoxorAIQueues                                       |
/// |                                                          |
/// | Structs:                                                 |
/// |   - MarketMaker                                          |
/// |   - OrderInfo                                            |
/// |   - MarketMakerConfig                                    |
/// |                                                          |
/// | Functions:                                               |
/// |   - main                                                 |
/// |   - init_tracing                                         |
/// +----------------------------------------------------------+

/// Channel template for placing orders with instrument placeholder (*)
const MOXOR_ORDER_PLACE_CHANNEL: &str = "moxor.orders.*.place";

/// Channel template for canceling orders with instrument placeholder (*)
const ORDER_CANCEL_CHANNEL: &str = "moxor.orders.*.cancel";

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

/// Stores information about an order placed by the market maker
struct OrderInfo {
    /// Order ID
    id: Uuid,
    /// Instrument ID
    instrument_id: Uuid,
    /// Side of the order (buy/sell)
    side: Side,
    /// Price of the order
    price: i64,
    /// Amount of the order
    amount: u64,
}

/// Configuration for the market maker
struct MarketMakerConfig {
    /// Number of price levels to maintain on each side
    levels: usize,
    /// Space between price levels (in ticks)
    level_spacing: i64,
    /// Size of orders to place
    order_size: u64,
    /// Base price for the market (theoretical price)
    base_price: i64,
    /// Account ID to use for orders
    account_id: Uuid,
    /// How often to refresh orders (in milliseconds)
    refresh_interval_ms: u64,
}

impl Default for MarketMakerConfig {
    fn default() -> Self {
        Self {
            levels: 5,
            level_spacing: 100,
            order_size: 1000,
            base_price: 10000,
            account_id: Uuid::new_v4(),
            refresh_interval_ms: 5000,
        }
    }
}

/// Market maker that connects to the matching engine via RabbitMQ
struct MarketMaker {
    /// RabbitMQ connection URL
    connection_url: String,
    /// Application ID
    app_id: String,
    /// Publisher for sending order messages
    publisher: Arc<Mutex<Option<rabbitmq::Publisher>>>,
    /// Instruments to make markets on
    instruments: Vec<Uuid>,
    /// Configuration for the market maker
    config: MarketMakerConfig,
    /// Active orders placed by the market maker
    active_orders: Arc<Mutex<HashMap<Uuid, OrderInfo>>>,
}

impl MarketMaker {
    /// Creates a new market maker
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration
    ///
    /// # Returns
    ///
    /// A new MarketMaker instance
    pub fn new(config: &Config) -> Self {
        Self {
            connection_url: config.rabbit_url.clone(),
            app_id: config.app_id.clone(),
            publisher: Arc::new(Mutex::new(None)),
            instruments: config.instruments.clone(),
            config: MarketMakerConfig::default(),
            active_orders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Connects to RabbitMQ
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn connect(&self) -> Result<(), RabbitMQError> {
        info!("Connecting to RabbitMQ at: {}", self.connection_url);
        
        // Perform pre-connection diagnostics
        self.diagnose_connection(&self.connection_url).await;
        
        info!("Building RabbitMQ server with a 10-second timeout...");
        
        // Try with retry and timeout
        let mut retry_count = 0;
        let max_retries = 3;
        let mut server_result = None;
        
        while retry_count < max_retries {
            info!("Connection attempt {} of {}", retry_count + 1, max_retries);
            
            // Create a new builder for each attempt with publisher configuration
            let builder = RabbitMQBuilder::new(&self.connection_url, &self.app_id)
                .publisher(MoxorAIQueues::MoxorAI, PublisherMode::Topic);
            
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
        
        let client = match server_result.unwrap() {
            Ok(c) => {
                info!("Successfully connected to RabbitMQ and built client");
                c
            },
            Err(err) => {
                error!("Failed to connect to RabbitMQ: {}", err);
                self.perform_connection_diagnostics(&self.connection_url).await;
                return Err(err);
            }
        };
        
        // Save the publisher for later use
        let mut publishers = client.get_publishers();
        
        if let Ok(publisher) = publishers.take_ownership((MoxorAIQueues::MoxorAI, PublisherMode::Topic)) {
            let mut publisher_lock = self.publisher.lock().await;
            *publisher_lock = Some(publisher);
            info!("Successfully obtained publisher");
        } else {
            error!("Failed to get publisher");
            return Err(RabbitMQError::ConnectionError("Failed to get publisher".to_string()));
        }
        
        info!("Successfully connected to RabbitMQ");
        Ok(())
    }

    /// Generates a random price variation
    ///
    /// # Returns
    ///
    /// A random price adjustment
    fn get_price_variation(&self) -> i64 {
        let mut rng = thread_rng();
        rng.gen_range(-200..=200)
    }

    /// Creates place order requests for market making
    ///
    /// # Arguments
    ///
    /// * `instrument` - Instrument ID
    ///
    /// # Returns
    ///
    /// A vector of PlaceOrderRequest objects
    async fn create_place_requests(&self, instrument: Uuid) -> Vec<PlaceOrderRequest> {
        let mut requests = Vec::new();
        let variation = self.get_price_variation();
        let base_price = self.config.base_price + variation;
        
        // Generate buy orders (bids)
        for i in 0..self.config.levels {
            let price = base_price - ((i as i64 + 1) * self.config.level_spacing);
            let order_id = Uuid::new_v4();
            
            let place_request = PlaceOrderRequest {
                version: 1,
                request_type: "place".to_string(),
                instrument,
                new_order_id: order_id,
                account_id: self.config.account_id,
                side: Side::Bid,
                order_type: OrderType::Limit,
                limit_price: Some(price),
                base_amount: self.config.order_size,
                trigger_price: None,
                time_in_force: TimeInForce::GTC,
                ext_id: Some(format!("mm-bid-{}", i)),
            };
            
            // Store the order info
            let mut active_orders = self.active_orders.lock().await;
            active_orders.insert(order_id, OrderInfo {
                id: order_id,
                instrument_id: instrument,
                side: Side::Bid,
                price,
                amount: self.config.order_size,
            });
            
            requests.push(place_request);
        }
        
        // Generate sell orders (asks)
        for i in 0..self.config.levels {
            let price = base_price + ((i as i64 + 1) * self.config.level_spacing);
            let order_id = Uuid::new_v4();
            
            let place_request = PlaceOrderRequest {
                version: 1,
                request_type: "place".to_string(),
                instrument,
                new_order_id: order_id,
                account_id: self.config.account_id,
                side: Side::Ask,
                order_type: OrderType::Limit,
                limit_price: Some(price),
                base_amount: self.config.order_size,
                trigger_price: None,
                time_in_force: TimeInForce::GTC,
                ext_id: Some(format!("mm-ask-{}", i)),
            };
            
            // Store the order info
            let mut active_orders = self.active_orders.lock().await;
            active_orders.insert(order_id, OrderInfo {
                id: order_id,
                instrument_id: instrument,
                side: Side::Ask,
                price,
                amount: self.config.order_size,
            });
            
            requests.push(place_request);
        }
        
        requests
    }

    /// Creates cancel order requests for existing orders
    ///
    /// # Returns
    ///
    /// A vector of CancelOrderRequest objects
    async fn create_cancel_requests(&self) -> Vec<CancelOrderRequest> {
        let active_orders = self.active_orders.lock().await;
        let mut requests = Vec::new();
        
        for (order_id, order_info) in active_orders.iter() {
            let cancel_request = CancelOrderRequest {
                version: 1,
                request_type: "cancel".to_string(),
                instrument: order_info.instrument_id,
                order_id: *order_id,
            };
            
            requests.push(cancel_request);
        }
        
        requests
    }

    /// Publishes an order placement request
    ///
    /// # Arguments
    ///
    /// * `request` - Order placement request
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    async fn publish_place_request(&self, request: PlaceOrderRequest) -> Result<(), RabbitMQError> {
        let publisher_lock = self.publisher.lock().await;
        
        if let Some(publisher) = &*publisher_lock {
            let channel = MOXOR_ORDER_PLACE_CHANNEL.replace('*', &request.instrument.to_string());
            let payload = serde_json::to_vec(&request).unwrap();
            
            // Create a message with the channel as topic since we're using the Topic exchange mode
            let message = rabbitmq::Message::new(payload, Some(channel));
            
            // Create a context with the current timestamp as request ID
            let ctx = rabbitmq::PublisherContext::new(
                &Utc::now().timestamp_millis().to_string(),
                None,
            );
            
            // Publish the message
            publisher.publish(message, ctx)?;
                
            info!("Published place order request for {} ({:?} at {})", 
                request.new_order_id, 
                request.side, 
                request.limit_price.unwrap_or(0));
        } else {
            error!("No publisher available");
            return Err(RabbitMQError::ConnectionError("No publisher available".to_string()));
        }
        
        Ok(())
    }

    /// Publishes an order cancellation request
    ///
    /// # Arguments
    ///
    /// * `request` - Order cancellation request
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    async fn publish_cancel_request(&self, request: CancelOrderRequest) -> Result<(), RabbitMQError> {
        let publisher_lock = self.publisher.lock().await;
        
        if let Some(publisher) = &*publisher_lock {
            let channel = ORDER_CANCEL_CHANNEL.replace('*', &request.instrument.to_string());
            let payload = serde_json::to_vec(&request).unwrap();
            
            // Create a message with the channel as topic since we're using the Topic exchange mode
            let message = rabbitmq::Message::new(payload, Some(channel));
            
            // Create a context with the current timestamp as request ID
            let ctx = rabbitmq::PublisherContext::new(
                &Utc::now().timestamp_millis().to_string(),
                None,
            );
            
            // Publish the message
            publisher.publish(message, ctx)?;
                
            info!("Published cancel order request for {}", request.order_id);
        } else {
            error!("No publisher available");
            return Err(RabbitMQError::ConnectionError("No publisher available".to_string()));
        }
        
        Ok(())
    }

    /// Refreshes all market making orders
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    async fn refresh_orders(&self) -> Result<(), RabbitMQError> {
        info!("Refreshing market maker orders");
        
        // First cancel all existing orders
        let cancel_requests = self.create_cancel_requests().await;
        
        for cancel_request in cancel_requests {
            match self.publish_cancel_request(cancel_request).await {
                Ok(_) => {},
                Err(e) => error!("Failed to cancel order: {}", e),
            }
        }
        
        // Clear the active orders after cancellation
        let mut active_orders = self.active_orders.lock().await;
        active_orders.clear();
        drop(active_orders);
        
        // Wait a moment for cancellations to process
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Create new orders for each instrument
        for instrument in &self.instruments {
            let place_requests = self.create_place_requests(*instrument).await;
            
            for place_request in place_requests {
                match self.publish_place_request(place_request).await {
                    Ok(_) => {},
                    Err(e) => error!("Failed to place order: {}", e),
                }
            }
        }
        
        Ok(())
    }

    /// Starts the market maker
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn start(&self) -> Result<(), RabbitMQError> {
        info!("Starting market maker for {} instruments", self.instruments.len());
        
        // Connect to RabbitMQ
        self.connect().await?;
        
        // Initial order placement
        match self.refresh_orders().await {
            Ok(_) => info!("Initial orders placed successfully"),
            Err(e) => error!("Failed to place initial orders: {}", e),
        }
        
        // Periodically refresh orders
        let refresh_interval = Duration::from_millis(self.config.refresh_interval_ms);
        let this = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            
            loop {
                interval.tick().await;
                
                match this.refresh_orders().await {
                    Ok(_) => info!("Orders refreshed successfully"),
                    Err(e) => error!("Failed to refresh orders: {}", e),
                }
            }
        });
        
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

impl Clone for MarketMaker {
    fn clone(&self) -> Self {
        Self {
            connection_url: self.connection_url.clone(),
            app_id: self.app_id.clone(),
            publisher: self.publisher.clone(),
            instruments: self.instruments.clone(),
            config: MarketMakerConfig {
                levels: self.config.levels,
                level_spacing: self.config.level_spacing,
                order_size: self.config.order_size,
                base_price: self.config.base_price,
                account_id: self.config.account_id,
                refresh_interval_ms: self.config.refresh_interval_ms,
            },
            active_orders: self.active_orders.clone(),
        }
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
    
    info!("Starting market maker...");

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

    // Create and start the market maker
    let market_maker = MarketMaker::new(&config);
    market_maker.start().await?;

    // Keep the main task alive 
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
} 
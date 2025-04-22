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

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::path::Path;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinSet;
use rand::{thread_rng, Rng};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use reqwest::{Client, StatusCode};
use anyhow::{Result, Context, anyhow};
use clap::Parser;

use ultimate_matching::domain::models::types::{Side, OrderType, TimeInForce};

/// Command line arguments for the market maker
#[derive(Parser, Debug)]
#[command(author, version, about = "High-performance market maker for the matching engine")]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "market_maker_config.json")]
    config: String,

    /// API endpoint URL
    #[arg(short, long, default_value = "http://localhost:3001")]
    api_url: String,

    /// Number of worker threads for sending orders
    #[arg(short, long, default_value = "16")]
    workers: usize,

    /// Orders per second target
    #[arg(short, long, default_value = "10000")]
    rate: usize,

    /// Instrument ID to trade (required unless in config)
    #[arg(short, long)]
    instrument: Option<String>,
    
    /// Trend factor (percentage increase per second)
    #[arg(short, long, default_value = "0.05")]
    trend: f64,
    
    /// Volatility factor (percentage of random price movement)
    #[arg(short, long, default_value = "2.0")]
    volatility: f64,
}

/// Configuration for the market maker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMakerConfig {
    /// Instrument ID to trade
    pub instrument_id: Option<Uuid>,
    
    /// Account ID to use for orders
    pub account_id: Uuid,
    
    /// Base price for order generation
    pub base_price: Decimal,
    
    /// Spread to maintain (as a percentage)
    pub spread_pct: Decimal,
    
    /// Maximum deviation from base price (as a percentage)
    pub max_deviation_pct: Decimal,
    
    /// Minimum order size
    pub min_order_size: Decimal,
    
    /// Maximum order size
    pub max_order_size: Decimal,
    
    /// Maximum number of active orders
    pub max_active_orders: usize,
    
    /// Order cancellation rate (0.0-1.0)
    pub cancel_rate: f64,
    
    /// Order placement rate (orders per second)
    pub orders_per_second: usize,
    
    /// Number of price levels to maintain
    pub price_levels: usize,
    
    /// Trend factor (percentage increase per second)
    pub trend_factor: Decimal,
    
    /// Volatility factor (percentage of random price movement)
    pub volatility_factor: Decimal,
}

impl MarketMakerConfig {
    /// Creates a new configuration with default values
    pub fn new(instrument_id: Uuid, account_id: Uuid) -> Self {
        Self {
            instrument_id: Some(instrument_id),
            account_id,
            base_price: dec!(100.0),
            spread_pct: dec!(0.1),
            max_deviation_pct: dec!(5.0),
            min_order_size: dec!(0.01),
            max_order_size: dec!(1.0),
            max_active_orders: 1000,
            cancel_rate: 0.3,
            orders_per_second: 10000,
            price_levels: 5,
            trend_factor: dec!(0.05), // 5% increase per second
            volatility_factor: dec!(2.0), // 2% volatility
        }
    }
    
    /// Loads configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let config = serde_json::from_reader(file)?;
        Ok(config)
    }
}

/// API client for interacting with the matching engine
#[derive(Debug, Clone)]
pub struct ApiClient {
    /// HTTP client
    client: Client,
    
    /// Base URL for the API
    base_url: String,
}

impl ApiClient {
    /// Creates a new API client
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client");
            
        Self {
            client,
            base_url: base_url.to_string(),
        }
    }
    
    /// Places an order through the API
    pub async fn place_order(&self, order: CreateOrderRequest) -> Result<OrderResponse> {
        let url = format!("{}/orders", self.base_url);
        
        let resp = self.client.post(&url)
            .json(&order)
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::CREATED => {
                let order_resp = resp.json::<OrderResponse>().await?;
                Ok(order_resp)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to place order: {} - {}", status, error_text))
            }
        }
    }
    
    /// Cancels an order through the API
    pub async fn cancel_order(&self, order_id: Uuid, instrument_id: Uuid) -> Result<OrderResponse> {
        let url = format!("{}/orders/{}", self.base_url, order_id);
        
        let resp = self.client.delete(&url)
            .query(&[("instrument_id", instrument_id.to_string())])
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::OK => {
                let order_resp = resp.json::<OrderResponse>().await?;
                Ok(order_resp)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to cancel order: {} - {}", status, error_text))
            }
        }
    }
    
    /// Gets market depth through the API
    pub async fn get_depth(&self, instrument_id: Uuid, levels: usize) -> Result<DepthResponse> {
        let url = format!("{}/instruments/{}/depth", self.base_url, instrument_id);
        
        let resp = self.client.get(&url)
            .query(&[("level", levels.to_string())])
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::OK => {
                let depth_resp = resp.json::<DepthResponse>().await?;
                Ok(depth_resp)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to get depth: {} - {}", status, error_text))
            }
        }
    }
    
    /// Ensures an instrument exists, creating it if necessary
    pub async fn ensure_instrument(&self, name: &str, base: &str, quote: &str) -> Result<Uuid> {
        // First try to list instruments
        let url = format!("{}/instruments", self.base_url);
        
        let resp = self.client.get(&url)
            .send()
            .await?;
            
        if resp.status() == StatusCode::OK {
            let instruments: Vec<InstrumentResponse> = resp.json().await?;
            
            // Check if our instrument already exists
            for instrument in instruments {
                if instrument.base_currency == base && instrument.quote_currency == quote {
                    return Ok(instrument.id);
                }
            }
        }
        
        // Create the instrument if not found
        let create_req = CreateInstrumentRequest {
            id: None,
            name: name.to_string(),
            base_currency: base.to_string(),
            quote_currency: quote.to_string(),
        };
        
        let resp = self.client.post(&url)
            .json(&create_req)
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::CREATED => {
                let instrument: InstrumentResponse = resp.json().await?;
                Ok(instrument.id)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to create instrument: {} - {}", status, error_text))
            }
        }
    }
}

/// Helper trait for decimal conversions
trait DecimalExt {
    fn to_f64(&self) -> Option<f64>;
    fn from_f64(val: f64) -> Option<Self> where Self: Sized;
}

impl DecimalExt for Decimal {
    fn to_f64(&self) -> Option<f64> {
        self.to_string().parse::<f64>().ok()
    }

    fn from_f64(val: f64) -> Option<Self> {
        // Use fully-qualified syntax to disambiguate
        <Decimal as num_traits::FromPrimitive>::from_f64(val)
    }
}

/// Order generator for the market maker
#[derive(Debug)]
pub struct OrderGenerator {
    /// Configuration for order generation
    config: MarketMakerConfig,
    
    /// Current market base price
    current_price: Decimal,
    
    /// Random number generator
    rng: rand::rngs::ThreadRng,
    
    /// Trend factor (percentage increase per update)
    trend_factor: Decimal,
    
    /// Volatility factor (percentage of random price movement)
    volatility_factor: Decimal,
    
    /// Last update timestamp
    last_update: Instant,
}

impl OrderGenerator {
    /// Creates a new order generator
    pub fn new(config: MarketMakerConfig) -> Self {
        Self {
            current_price: config.base_price,
            config: config.clone(),
            rng: thread_rng(),
            trend_factor: config.trend_factor,
            volatility_factor: config.volatility_factor,
            last_update: Instant::now(),
        }
    }
    
    /// Updates the current market price
    pub fn update_price(&mut self, price: Decimal) {
        // Calculate time elapsed since last update
        let elapsed = self.last_update.elapsed().as_secs_f64();
        self.last_update = Instant::now();
        
        // Apply trend (upward movement)
        let trend_increase = self.trend_factor * DecimalExt::from_f64(elapsed).unwrap_or(Decimal::ZERO);
        let trended_price = price * (Decimal::ONE + trend_increase);
        
        // Apply volatility (random up/down movement)
        let volatility = self.add_volatility(trended_price);
        
        // Update the current price
        self.current_price = volatility;
    }
    
    /// Adds volatility to the price
    fn add_volatility(&mut self, price: Decimal) -> Decimal {
        // Generate a random value between -1.0 and 1.0
        let rand_val = self.rng.gen_range(-1.0..1.0);
        
        // Calculate the volatility factor
        let volatility_pct = self.volatility_factor.to_f64().unwrap_or(2.0);
        let volatility_factor = rand_val * volatility_pct / 100.0;
        
        // Apply volatility to the price
        let decimal_volatility = DecimalExt::from_f64(volatility_factor).unwrap_or(Decimal::ZERO);
        price * (Decimal::ONE + decimal_volatility)
    }
    
    /// Adds random noise to the price
    fn add_price_noise(&mut self, price: Decimal) -> Decimal {
        // Generate a random value between -0.5 and 0.5
        let rand_val = self.rng.gen_range(-0.5..0.5);
        
        // Calculate the noise factor
        let max_dev_pct = self.config.max_deviation_pct.to_f64().unwrap_or(5.0);
        let noise_factor = rand_val * 2.0 * max_dev_pct / 100.0;
        
        // Apply noise to the price
        let decimal_noise = DecimalExt::from_f64(noise_factor).unwrap_or(Decimal::ZERO);
        price * (Decimal::ONE + decimal_noise)
    }
    
    /// Generates a random order size
    fn generate_order_size(&mut self) -> Decimal {
        let min = self.config.min_order_size.to_f64().unwrap_or(0.01);
        let max = self.config.max_order_size.to_f64().unwrap_or(1.0);
        
        // Generate a random value between min and max
        let size = min + self.rng.gen_range(0.0..1.0) * (max - min);
        DecimalExt::from_f64(size).unwrap_or(self.config.min_order_size)
    }
    
    /// Generates orders for market making
    pub fn generate_orders(&mut self, count: usize) -> Vec<CreateOrderRequest> {
        let mut orders = Vec::with_capacity(count);
        
        let half_spread = self.config.spread_pct / Decimal::from(200);
        let bid_price = self.current_price * (Decimal::ONE - half_spread);
        let ask_price = self.current_price * (Decimal::ONE + half_spread);
        
        let orders_per_side = count / 2;
        let price_step = self.config.max_deviation_pct / Decimal::from(self.config.price_levels);
        
        // Generate buy orders
        for i in 0..orders_per_side {
            let level_offset = Decimal::from(i % self.config.price_levels) * price_step / Decimal::from(100);
            let price = self.add_price_noise(bid_price * (Decimal::ONE - level_offset));
            
            orders.push(CreateOrderRequest {
                ext_id: Some(format!("mm-{}", Uuid::new_v4())),
                account_id: self.config.account_id,
                order_type: OrderType::Limit,
                instrument_id: self.config.instrument_id.expect("Instrument ID must be set"),
                side: Side::Bid,
                limit_price: Some(price),
                trigger_price: None,
                base_amount: self.generate_order_size(),
                time_in_force: TimeInForce::GTC,
            });
        }
        
        // Generate sell orders
        for i in 0..count - orders_per_side {
            let level_offset = Decimal::from(i % self.config.price_levels) * price_step / Decimal::from(100);
            let price = self.add_price_noise(ask_price * (Decimal::ONE + level_offset));
            
            orders.push(CreateOrderRequest {
                ext_id: Some(format!("mm-{}", Uuid::new_v4())),
                account_id: self.config.account_id,
                order_type: OrderType::Limit,
                instrument_id: self.config.instrument_id.expect("Instrument ID must be set"),
                side: Side::Ask,
                limit_price: Some(price),
                trigger_price: None,
                base_amount: self.generate_order_size(),
                time_in_force: TimeInForce::GTC,
            });
        }
        
        orders
    }
}

/// Main market maker implementation
pub struct MarketMaker {
    /// Configuration
    config: MarketMakerConfig,
    
    /// API client
    api_client: ApiClient,
    
    /// Order generator
    order_generator: OrderGenerator,
    
    /// Active orders map
    active_orders: HashMap<Uuid, OrderResponse>,
    
    /// Maximum number of concurrent requests
    max_concurrent_requests: usize,
    
    /// Order rate throttling (orders per second)
    orders_per_second: usize,
}

impl MarketMaker {
    /// Creates a new market maker
    pub fn new(config: MarketMakerConfig, api_url: &str) -> Self {
        let api_client = ApiClient::new(api_url);
        let order_generator = OrderGenerator::new(config.clone());
        
        Self {
            api_client,
            order_generator,
            orders_per_second: config.orders_per_second,
            config,
            active_orders: HashMap::with_capacity(1000),
            max_concurrent_requests: 1000,
        }
    }
    
    /// Sets the target orders per second
    pub fn set_orders_per_second(&mut self, rate: usize) {
        self.orders_per_second = rate;
    }
    
    /// Runs the market maker in a loop
    pub async fn run(&mut self) -> Result<()> {
        println!("Starting market maker for instrument: {}", self.config.instrument_id.expect("Instrument ID must be set"));
        println!("Target order rate: {} orders/second", self.orders_per_second);
        
        // Initial price sync
        self.sync_market_price().await?;
        
        let (order_tx, order_rx) = mpsc::channel::<CreateOrderRequest>(10000);
        let (cancel_tx, cancel_rx) = mpsc::channel::<Uuid>(10000);
        
        // Spawn order processor tasks
        let _order_processor = self.spawn_order_processor(order_rx);
        let _cancel_processor = self.spawn_cancel_processor(cancel_rx);
        
        // Track time to maintain order rate
        let mut last_update = Instant::now();
        let mut last_price_update = Instant::now();
        let mut orders_sent = 0;
        let mut _cancels_sent = 0;
        let rate_window_ms = 1000;
        let price_update_interval_ms = 5000; // Update price every 5 seconds
        
        loop {
            // Generate orders for this cycle
            // Calculate how many orders to generate based on time since last update
            let elapsed_ms = last_update.elapsed().as_millis() as u64;
            if elapsed_ms >= rate_window_ms {
                // Calculate orders per second
                let actual_rate = (orders_sent as f64) / (elapsed_ms as f64 / 1000.0);
                println!("Order rate: {:.2} orders/second, Active orders: {}", 
                    actual_rate, self.active_orders.len());
                
                // Reset counters
                orders_sent = 0;
                _cancels_sent = 0;
                last_update = Instant::now();
                
                // Sync market price occasionally
                self.sync_market_price().await?;
            }
            
            // Update price periodically to maintain trend
            let price_elapsed_ms = last_price_update.elapsed().as_millis() as u64;
            if price_elapsed_ms >= price_update_interval_ms {
                self.update_price_trend().await?;
                last_price_update = Instant::now();
            }
            
            // Calculate batch size based on target rate and time since last batch
            let time_factor = elapsed_ms as f64 / rate_window_ms as f64;
            let target_orders = (self.orders_per_second as f64 * time_factor) as usize;
            let orders_to_send = target_orders.saturating_sub(orders_sent);
            
            // Process order cancellations first
            if self.active_orders.len() > self.config.max_active_orders || 
               self.rng_check(self.config.cancel_rate) {
                let to_cancel = self.select_orders_to_cancel();
                for order_id in to_cancel {
                    if cancel_tx.send(order_id).await.is_ok() {
                        _cancels_sent += 1;
                    }
                }
            }
            
            // Generate and send new orders
            if orders_to_send > 0 {
                let orders = self.order_generator.generate_orders(orders_to_send);
                for order in orders {
                    if order_tx.send(order).await.is_ok() {
                        orders_sent += 1;
                    }
                }
            }
            
            // Sleep a small amount to prevent tight loop
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    
    /// Check random condition based on probability
    fn rng_check(&self, probability: f64) -> bool {
        let mut rng = thread_rng();
        rng.gen_range(0.0..1.0) < probability
    }
    
    /// Selects orders to cancel
    fn select_orders_to_cancel(&mut self) -> Vec<Uuid> {
        let mut to_cancel = Vec::new();
        let target_cancels = (self.active_orders.len() as f64 * self.config.cancel_rate) as usize;
        
        // If we have too many active orders, cancel some
        let excess = self.active_orders.len().saturating_sub(self.config.max_active_orders);
        let cancel_count = target_cancels.max(excess).min(100);
        
        if cancel_count == 0 {
            return to_cancel;
        }
        
        // Select random orders to cancel
        let order_ids: Vec<Uuid> = self.active_orders.keys().cloned().collect();
        let mut rng = thread_rng();
        
        for _ in 0..cancel_count {
            if order_ids.is_empty() {
                break;
            }
            
            let idx = rng.gen_range(0..order_ids.len());
            if idx < order_ids.len() {
                to_cancel.push(order_ids[idx]);
            }
        }
        
        to_cancel
    }
    
    /// Syncs the current market price from the API
    async fn sync_market_price(&mut self) -> Result<()> {
        let instrument_id = self.config.instrument_id.expect("Instrument ID must be set");
        let depth = self.api_client.get_depth(instrument_id, 1).await?;
        
        let bid = depth.bids.first().map(|level| level.price);
        let ask = depth.asks.first().map(|level| level.price);
        
        // Get the current price from the order generator
        let current_price = self.order_generator.current_price;
        
        // Calculate a new price based on market data and our trend
        let new_price = if let (Some(bid), Some(ask)) = (bid, ask) {
            // Use mid price as a reference point, but maintain our trend
            let mid_price = (bid + ask) / Decimal::from(2);
            
            // If market price is significantly different from our trend, adjust gradually
            let price_diff_pct = (mid_price - current_price).abs() / current_price * Decimal::from(100);
            
            if price_diff_pct > Decimal::from(10) {
                // Market price is significantly different, blend it with our trend
                // (70% our trend, 30% market price)
                current_price * Decimal::from(7) / Decimal::from(10) + mid_price * Decimal::from(3) / Decimal::from(10)
            } else {
                // Market price is close to our trend, continue with our trend
                current_price
            }
        } else if let Some(bid) = bid {
            // Similar logic for bid-only case
            let price_diff_pct = (bid - current_price).abs() / current_price * Decimal::from(100);
            
            if price_diff_pct > Decimal::from(10) {
                current_price * Decimal::from(7) / Decimal::from(10) + bid * Decimal::from(3) / Decimal::from(10)
            } else {
                current_price
            }
        } else if let Some(ask) = ask {
            // Similar logic for ask-only case
            let price_diff_pct = (ask - current_price).abs() / current_price * Decimal::from(100);
            
            if price_diff_pct > Decimal::from(10) {
                current_price * Decimal::from(7) / Decimal::from(10) + ask * Decimal::from(3) / Decimal::from(10)
            } else {
                current_price
            }
        } else {
            // No market data, continue with our trend
            current_price
        };
        
        // Update the price with our trend and volatility
        self.order_generator.update_price(new_price);
        
        Ok(())
    }
    
    /// Updates the price trend without syncing with the market
    async fn update_price_trend(&mut self) -> Result<()> {
        // Get the current price
        let current_price = self.order_generator.current_price;
        
        // Update the price with our trend and volatility
        self.order_generator.update_price(current_price);
        
        // Log the price trend
        println!("Price trend: {:.8}", self.order_generator.current_price);
        
        Ok(())
    }
    
    /// Spawns a task to process order placements
    fn spawn_order_processor(
        &self, 
        mut orders: mpsc::Receiver<CreateOrderRequest>
    ) -> tokio::task::JoinHandle<()> {
        let client = self.api_client.clone();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_requests));
        let active_orders = Arc::new(tokio::sync::RwLock::new(self.active_orders.clone()));
        
        tokio::spawn(async move {
            let mut join_set = JoinSet::new();
            
            while let Some(order) = orders.recv().await {
                let permit = match semaphore.clone().acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        eprintln!("Failed to acquire semaphore permit");
                        continue;
                    }
                };
                
                let client = client.clone();
                let active_orders = active_orders.clone();
                
                join_set.spawn(async move {
                    let result = client.place_order(order).await;
                    drop(permit); // Release the permit
                    
                    if let Ok(order_resp) = result {
                        let mut orders = active_orders.write().await;
                        orders.insert(order_resp.id, order_resp);
                    }
                });
            }
            
            // Wait for all tasks to complete
            while join_set.join_next().await.is_some() {}
        })
    }
    
    /// Spawns a task to process order cancellations
    fn spawn_cancel_processor(
        &self, 
        mut cancels: mpsc::Receiver<Uuid>
    ) -> tokio::task::JoinHandle<()> {
        let client = self.api_client.clone();
        let instrument_id = self.config.instrument_id.expect("Instrument ID must be set");
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_requests));
        let active_orders = Arc::new(tokio::sync::RwLock::new(self.active_orders.clone()));
        
        tokio::spawn(async move {
            let mut join_set = JoinSet::new();
            
            while let Some(order_id) = cancels.recv().await {
                let permit = match semaphore.clone().acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        eprintln!("Failed to acquire semaphore permit");
                        continue;
                    }
                };
                
                let client = client.clone();
                let instrument_id = instrument_id;
                let active_orders = active_orders.clone();
                
                join_set.spawn(async move {
                    let result = client.cancel_order(order_id, instrument_id).await;
                    drop(permit); // Release the permit
                    
                    let mut orders = active_orders.write().await;
                    orders.remove(&order_id);
                    
                    if let Err(e) = result {
                        if !e.to_string().contains("not found") {
                            eprintln!("Error cancelling order {}: {}", order_id, e);
                        }
                    }
                });
            }
            
            // Wait for all tasks to complete
            while join_set.join_next().await.is_some() {}
        })
    }
}

/// Data transfer objects
mod dto {
    use serde::{Serialize, Deserialize};
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;
    use uuid::Uuid;
    use ultimate_matching::domain::models::types::{Side, OrderType, OrderStatus, TimeInForce};

    /// Request to create a new order
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateOrderRequest {
        /// Optional external identifier provided by the client
        pub ext_id: Option<String>,
        /// Identifier for the account placing the order
        pub account_id: Uuid,
        /// Type of the order (Limit, Market, etc.)
        pub order_type: OrderType,
        /// Identifier for the instrument being traded
        pub instrument_id: Uuid,
        /// Side of the order (Buy or Sell)
        pub side: Side,
        /// Limit price for Limit/StopLimit orders
        pub limit_price: Option<Decimal>,
        /// Trigger price for Stop/StopLimit orders
        pub trigger_price: Option<Decimal>,
        /// Initial order quantity in base units
        pub base_amount: Decimal,
        /// Time-in-force policy for the order
        #[serde(default)]
        pub time_in_force: TimeInForce,
    }

    /// Response for an order
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OrderResponse {
        /// Unique identifier for the order
        pub id: Uuid,
        /// Optional external identifier provided by the client
        pub ext_id: Option<String>,
        /// Identifier for the account that placed the order
        pub account_id: Uuid,
        /// Type of the order
        pub order_type: OrderType,
        /// Identifier for the instrument being traded
        pub instrument_id: Uuid,
        /// Side of the order
        pub side: Side,
        /// Limit price for Limit/StopLimit orders
        pub limit_price: Option<Decimal>,
        /// Trigger price for Stop/StopLimit orders
        pub trigger_price: Option<Decimal>,
        /// Initial order quantity in base units
        pub base_amount: Decimal,
        /// Remaining quantity in base units
        pub remaining_base: Decimal,
        /// Filled quantity in base units
        pub filled_base: Decimal,
        /// Filled quantity in quote units
        pub filled_quote: Decimal,
        /// Current status of the order
        pub status: OrderStatus,
        /// Creation timestamp
        pub created_at: DateTime<Utc>,
        /// Last update timestamp
        pub updated_at: DateTime<Utc>,
    }

    /// Price level in the depth response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PriceLevelResponse {
        /// Price for this level
        pub price: Decimal,
        /// Total volume at this price level
        pub volume: Decimal,
        /// Number of orders at this price level
        pub order_count: u32,
    }

    /// Response for order book depth
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DepthResponse {
        /// Bid side price levels (descending order by price)
        pub bids: Vec<PriceLevelResponse>,
        /// Ask side price levels (ascending order by price)
        pub asks: Vec<PriceLevelResponse>,
        /// Timestamp of the snapshot
        pub timestamp: DateTime<Utc>,
        /// Instrument ID
        pub instrument_id: Uuid,
    }

    /// Request to create a new instrument
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateInstrumentRequest {
        /// Optional specific ID for the instrument (random if not provided)
        pub id: Option<Uuid>,
        /// Human-readable name for the instrument
        pub name: String,
        /// Base currency symbol
        pub base_currency: String,
        /// Quote currency symbol
        pub quote_currency: String,
    }

    /// Response for an instrument
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InstrumentResponse {
        /// Unique identifier for the instrument
        pub id: Uuid,
        /// Human-readable name for the instrument
        pub name: String,
        /// Base currency symbol
        pub base_currency: String,
        /// Quote currency symbol
        pub quote_currency: String,
    }
}

// Use our DTO module
use dto::*;

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Validate arguments
    if args.workers == 0 {
        return Err(anyhow!("Workers must be greater than 0"));
    }
    
    if args.rate == 0 {
        return Err(anyhow!("Rate must be greater than 0"));
    }
    
    // Set up the runtime for optimal performance
    let num_threads = args.workers;
    println!("Setting up {} worker threads", num_threads);
    
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_threads)
        .thread_name("market-maker-worker")
        .thread_stack_size(2 * 1024 * 1024) // 2MB stack
        .enable_all()
        .build()?
        .block_on(run_market_maker(args))
}

async fn run_market_maker(args: Args) -> Result<()> {
    println!("Market Maker starting up...");
    println!("API URL: {}", args.api_url);
    println!("Target rate: {} orders/second", args.rate);
    println!("Trend factor: {}%/second", args.trend);
    println!("Volatility factor: {}%", args.volatility);
    
    // Initialize API client
    let api_client = ApiClient::new(&args.api_url);
    
    // Load or create config
    let config = if let Ok(mut config) = MarketMakerConfig::from_file(&args.config) {
        println!("Loaded configuration from {}", args.config);
        
        // Override trend and volatility with command-line arguments
        config.trend_factor = DecimalExt::from_f64(args.trend).unwrap_or(dec!(0.05));
        config.volatility_factor = DecimalExt::from_f64(args.volatility).unwrap_or(dec!(2.0));
        
        // Verify the instrument exists even when loaded from config
        if let Some(instrument_id) = config.instrument_id {
            println!("Verifying instrument {} exists...", instrument_id);
            
            // Check if instrument exists by trying to get depth
            match api_client.get_depth(instrument_id, 1).await {
                // Instrument exists, continue with loaded config
                Ok(_) => {
                    println!("Instrument verified successfully");
                },
                // Instrument doesn't exist, create a new one
                Err(e) => {
                    println!("Instrument verification failed: {}", e);
                    println!("Creating a new instrument...");
                    
                    // Create a new instrument
                    let new_instrument_id = api_client.ensure_instrument("BTC-USD", "BTC", "USD").await
                        .context("Failed to ensure instrument exists")?;
                    
                    // Update config with new instrument ID
                    config.instrument_id = Some(new_instrument_id);
                    println!("Created new instrument: {}", new_instrument_id);
                    
                    // Save updated config
                    let config_json = serde_json::to_string_pretty(&config)?;
                    std::fs::write(&args.config, config_json)?;
                    println!("Updated configuration in {}", args.config);
                }
            }
        } else {
            // No instrument ID in config, create one
            let instrument_id = api_client.ensure_instrument("BTC-USD", "BTC", "USD").await
                .context("Failed to ensure instrument exists")?;
            config.instrument_id = Some(instrument_id);
        }
        
        config
    } else {
        println!("No configuration file found, using defaults");
        
        // Get or create instrument
        let instrument_id = if let Some(instrument_str) = args.instrument {
            if let Ok(id) = Uuid::parse_str(&instrument_str) {
                id
            } else {
                api_client.ensure_instrument("BTC-USD", "BTC", "USD").await
                    .context("Failed to ensure instrument exists")?
            }
        } else {
            api_client.ensure_instrument("BTC-USD", "BTC", "USD").await
                .context("Failed to ensure instrument exists")?
        };
        
        // Create a default config
        let account_id = Uuid::new_v4();
        println!("Using instrument ID: {} and account ID: {}", instrument_id, account_id);
        
        let mut config = MarketMakerConfig::new(instrument_id, account_id);
        config.orders_per_second = args.rate;
        config.trend_factor = DecimalExt::from_f64(args.trend).unwrap_or(dec!(0.05));
        config.volatility_factor = DecimalExt::from_f64(args.volatility).unwrap_or(dec!(2.0));
        
        // Save the config for future use
        let config_json = serde_json::to_string_pretty(&config)?;
        std::fs::write(&args.config, config_json)?;
        println!("Saved configuration to {}", args.config);
        
        config
    };
    
    // Create and run the market maker
    let mut market_maker = MarketMaker::new(config, &args.api_url);
    
    // Set the target order rate
    market_maker.set_orders_per_second(args.rate);
    
    // Run the market maker
    market_maker.run().await?;
    
    Ok(())
} 
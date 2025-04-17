//--------------------------------------------------------------------------------------------------
// NEUTRAL MARKET MAKER
//--------------------------------------------------------------------------------------------------
// This module implements a neutral market maker that connects to the matching engine API
// and places orders with a market making strategy to profit from the spread.
//
// | Component                | Description                                                |
// |--------------------------|-----------------------------------------------------------|
// | NeutralMarketMaker       | Core market maker implementation                          |
// | MarketMakerConfig        | Configuration for the market maker                        |
// | MarketMakingStrategy     | Strategy for determining prices and quantities            |
// | InventoryManager         | Manages the market maker's inventory                      |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name                    | Description                                       | Key Methods       |
// |-------------------------|---------------------------------------------------|------------------|
// | NeutralMarketMaker      | Main market maker component                       | run, start        |
// | MarketMakerConfig       | Configuration parameters                          | from_file, new    |
// | OrderGenerator          | Generates orders based on strategy                | generate_orders   |
// | ApiClient               | Client for the matching engine API                | place_order       |
// | InventoryManager        | Manages inventory and risk                        | update_position   |
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
use std::collections::VecDeque;
use num_traits::{ToPrimitive, FromPrimitive};
use std::collections::BTreeMap;

use ultimate_matching::types::{Side, OrderType, TimeInForce, Order, OrderStatus};
use ultimate_matching::api::{
    CreateOrderRequest, OrderResponse, DepthResponse, TradeResponse,
    InstrumentResponse, CreateInstrumentRequest
};

/// Command line arguments for the market maker
#[derive(Parser, Debug)]
#[command(author, version, about = "Neutral market maker for the matching engine")]
struct Args {
    /// Path to the configuration file
    #[arg(short = 'f', long, default_value = "market_maker_config.json")]
    config: String,

    /// API endpoint URL
    #[arg(short, long, default_value = "http://localhost:3001")]
    api_url: String,

    /// Number of worker threads for sending orders
    #[arg(short, long, default_value = "16")]
    workers: usize,

    /// Orders per second target
    #[arg(short, long, default_value = "1000")]
    rate: usize,

    /// Instrument ID to trade (required unless in config)
    #[arg(short = 'I', long)]
    instrument: Option<String>,
    
    /// Base spread as percentage of price
    #[arg(short = 's', long, default_value = "0.1")]
    spread: f64,
    
    /// Trading capital in quote currency
    #[arg(short = 'C', long, default_value = "100000.0")]
    capital: f64,
    
    /// Maximum inventory imbalance as percentage of capital
    #[arg(short = 'm', long, default_value = "5.0")]
    max_imbalance: f64,
}

/// Configuration for the neutral market maker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMakerConfig {
    /// Instrument ID to trade
    pub instrument_id: Option<Uuid>,
    
    /// Account ID to use for orders
    pub account_id: Uuid,
    
    /// Base price for order generation
    pub base_price: Decimal,
    
    /// Base spread as percentage (e.g., 0.1 for 0.1%)
    pub base_spread_pct: Decimal,
    
    /// Minimum order size
    pub min_order_size: Decimal,
    
    /// Maximum order size
    pub max_order_size: Decimal,
    
    /// Maximum number of active orders
    pub max_active_orders: usize,
    
    /// Number of price levels to maintain
    pub price_levels: usize,
    
    /// Order refresh interval (in milliseconds)
    pub refresh_interval_ms: u64,
    
    /// Skew factor for inventory management (0.0-1.0)
    /// Higher values adjust quotes more aggressively when inventory is imbalanced
    pub inventory_skew_factor: Decimal,
    
    /// Trading capital in quote currency
    pub capital: Decimal,
    
    /// Maximum inventory imbalance as percentage of capital
    pub max_imbalance_pct: Decimal,
    
    /// Minimum profit target per trade (as percentage)
    pub min_profit_pct: Decimal,
    
    /// Volatility scaling factor (adjusts spread in volatile markets)
    pub volatility_factor: Decimal,
}

impl MarketMakerConfig {
    /// Creates a new configuration with default values
    pub fn new(instrument_id: Uuid, account_id: Uuid) -> Self {
        Self {
            instrument_id: Some(instrument_id),
            account_id,
            base_price: dec!(100.0),
            base_spread_pct: dec!(0.1),
            min_order_size: dec!(0.01),
            max_order_size: dec!(1.0),
            max_active_orders: 100,
            price_levels: 5,
            refresh_interval_ms: 1000,
            inventory_skew_factor: dec!(0.2),
            capital: dec!(100000.0),
            max_imbalance_pct: dec!(5.0),
            min_profit_pct: dec!(0.05),
            volatility_factor: dec!(1.0),
        }
    }
    
    /// Loads configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let config = serde_json::from_reader(file)?;
        Ok(config)
    }
}

/// Tracks market state and statistics
#[derive(Debug, Clone)]
struct MarketState {
    /// Current mid price
    mid_price: Decimal,
    
    /// Current spread
    spread: Decimal,
    
    /// Recent trade prices
    trade_prices: VecDeque<Decimal>,
    
    /// Recent trade volumes
    trade_volumes: VecDeque<Decimal>,
    
    /// Order flow imbalance (positive for more buys, negative for more sells)
    order_flow_imbalance: f64,
    
    /// Current position size (positive for long, negative for short)
    position_size: Decimal,
    
    /// Current exposure (in quote currency)
    exposure: Decimal,
    
    /// Last update timestamp
    last_update: Instant,
}

impl MarketState {
    /// Creates a new market state
    fn new(initial_price: Decimal, initial_spread: Decimal) -> Self {
        Self {
            mid_price: initial_price,
            spread: initial_spread,
            trade_prices: VecDeque::new(),
            trade_volumes: VecDeque::new(),
            order_flow_imbalance: 0.0,
            position_size: Decimal::ZERO,
            exposure: Decimal::ZERO,
            last_update: Instant::now(),
        }
    }
    
    /// Updates the market state with a new trade
    fn update_with_trade(&mut self, price: Decimal, volume: Decimal, side: Side) {
        // Update trade history
        self.trade_prices.push_back(price);
        if self.trade_prices.len() > 1000 {
            self.trade_prices.pop_front();
        }
        
        self.trade_volumes.push_back(volume);
        if self.trade_volumes.len() > 1000 {
            self.trade_volumes.pop_front();
        }
        
        // Update order flow imbalance
        let imbalance_change = match side {
            Side::Bid => 1.0,
            Side::Ask => -1.0,
        };
        self.order_flow_imbalance = (self.order_flow_imbalance * 0.9) + (imbalance_change * 0.1);
        
        // Update position and exposure
        let position_change = match side {
            Side::Bid => volume,
            Side::Ask => -volume,
        };
        self.position_size += position_change;
        self.exposure = self.position_size * self.mid_price;
        
        self.last_update = Instant::now();
    }
    
    /// Calculates current volatility
    fn calculate_volatility(&self, window: usize) -> Decimal {
        if self.trade_prices.len() < 2 {
            return Decimal::ZERO;
        }
        
        let window = window.min(self.trade_prices.len());
        let prices: Vec<Decimal> = self.trade_prices.iter().rev().take(window).cloned().collect();
        
        let mean = prices.iter().sum::<Decimal>() / Decimal::from(prices.len());
        let variance = prices.iter()
            .map(|p| {
                let diff = *p - mean;
                diff * diff  // Square the difference instead of using pow
            })
            .sum::<Decimal>() / Decimal::from(prices.len());
        
        variance.sqrt().unwrap_or(Decimal::ZERO)
    }
    
    /// Adjusts spread based on market conditions
    fn adjust_spread(&mut self, config: &MarketMakerConfig) {
        let volatility = self.calculate_volatility(config.price_levels);
        let volatility_factor = volatility / self.mid_price;
        
        // Increase spread with volatility
        let spread_adjustment = volatility_factor * config.volatility_factor;
        
        // Adjust for order flow imbalance
        let imbalance_factor = Decimal::from_f64_retain(self.order_flow_imbalance.abs()).unwrap_or(Decimal::ZERO) * config.inventory_skew_factor;
        
        // Calculate new spread
        let new_spread = self.mid_price * (config.base_spread_pct + spread_adjustment + imbalance_factor);
        
        // Clamp spread within limits
        self.spread = new_spread.max(self.mid_price * config.min_order_size)
            .min(self.mid_price * config.max_order_size);
    }
}

/// Main neutral market maker implementation
pub struct NeutralMarketMaker {
    /// Configuration
    config: MarketMakerConfig,
    
    /// API client
    api_client: ApiClient,
    
    /// Market making strategy
    strategy: MarketMakingStrategy,
    
    /// Inventory manager
    inventory_manager: InventoryManager,
    
    /// Active orders map
    active_orders: HashMap<Uuid, OrderResponse>,
    
    /// Map of order IDs to sides for easier fill processing
    order_sides: HashMap<Uuid, Side>,
    
    /// Maximum number of concurrent requests
    max_concurrent_requests: usize,
    
    /// Order refresh interval
    refresh_interval: Duration,
    
    /// Last order refresh time
    last_refresh: Instant,
    
    /// Random number generator
    rng: rand::rngs::ThreadRng,
}

impl NeutralMarketMaker {
    /// Creates a new neutral market maker
    pub fn new(config: MarketMakerConfig, api_url: &str) -> Self {
        let api_client = ApiClient::new(api_url);
        let strategy = MarketMakingStrategy::new(config.clone());
        let inventory_manager = InventoryManager::new(
            config.capital, 
            config.max_imbalance_pct
        );
        
        Self {
            config: config.clone(),
            api_client,
            strategy,
            inventory_manager,
            active_orders: HashMap::with_capacity(config.max_active_orders * 2),
            order_sides: HashMap::with_capacity(config.max_active_orders * 2),
            max_concurrent_requests: 100,
            refresh_interval: Duration::from_millis(config.refresh_interval_ms),
            last_refresh: Instant::now(),
            rng: thread_rng(),
        }
    }
    
    /// Runs the market maker in a loop
    pub async fn run(&mut self) -> Result<()> {
        println!("Starting neutral market maker for instrument: {}", 
                 self.config.instrument_id.expect("Instrument ID must be set"));
        println!("Strategy: Market making with inventory management");
        println!("Base spread: {}%", self.config.base_spread_pct);
        println!("Price levels: {}", self.config.price_levels);
        println!("Refresh interval: {}ms", self.config.refresh_interval_ms);
        
        // Initial market sync
        self.sync_market_state().await?;
        
        let (order_tx, order_rx) = mpsc::channel::<CreateOrderRequest>(1000);
        let (cancel_tx, cancel_rx) = mpsc::channel::<Uuid>(1000);
        let (fill_tx, fill_rx) = mpsc::channel::<(Side, Decimal, Decimal)>(1000);
        
        // Spawn processor tasks
        let _order_processor = self.spawn_order_processor(order_rx, fill_tx.clone());
        let _cancel_processor = self.spawn_cancel_processor(cancel_rx);
        let _fill_processor = self.spawn_fill_processor(fill_rx);
        
        // Main loop
        loop {
            // Check if it's time to refresh orders
            if self.last_refresh.elapsed() >= self.refresh_interval {
                // Log current state
                self.log_market_maker_state().await;
                
                // Sync market state
                match self.sync_market_state().await {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Error syncing market state: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                }
                
                // Cancel existing orders
                self.cancel_all_orders(cancel_tx.clone()).await?;
                
                // Wait for cancellations to propagate
                tokio::time::sleep(Duration::from_millis(500)).await;
                
                // Generate and place new orders
                self.place_new_orders(order_tx.clone()).await?;
                
                // Update last refresh time
                self.last_refresh = Instant::now();
            }
            
            // Sleep for a bit to prevent tight looping
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    /// Syncs the current market state
    async fn sync_market_state(&mut self) -> Result<()> {
        let instrument_id = self.config.instrument_id.expect("Instrument ID must be set");
        
        // Get market depth
        let depth = match self.api_client.get_depth(instrument_id, 10).await {
            Ok(depth) => depth,
            Err(e) => {
                eprintln!("Error getting market depth: {}", e);
                return Err(e);
            }
        };
        
        // Get recent trades
        let trades = match self.api_client.get_trades(instrument_id, 20).await {
            Ok(trades) => trades,
            Err(e) => {
                eprintln!("Error getting recent trades: {}", e);
                return Err(e);
            }
        };
        
        // Extract best bid and ask
        let best_bid = depth.bids.first().map(|level| level.price);
        let best_ask = depth.asks.first().map(|level| level.price);
        
        // Update strategy with market data
        self.strategy.update_market_state(best_bid, best_ask, &trades);
        
        Ok(())
    }
    
    /// Logs the current state of the market maker
    async fn log_market_maker_state(&self) {
        // Get inventory value
        let mid_price = self.strategy.last_mid_price;
        let base_position = self.inventory_manager.base_position;
        let quote_position = self.inventory_manager.quote_position();
        let total_value = self.inventory_manager.total_value(mid_price);
        let realized_pnl = self.inventory_manager.realized_pnl();
        
        println!("\n======= MARKET MAKER STATE =======");
        println!("Mid Price: {:.8}", mid_price);
        println!("Base Position: {:.8} | Quote Position: {:.8}", base_position, quote_position);
        println!("Total Value: {:.8} | Initial Capital: {:.8}", total_value, self.config.capital);
        println!("Realized PnL: {:.8} ({:.4}%)", 
                 realized_pnl, 
                 realized_pnl * Decimal::from(100) / self.config.capital);
        println!("Active Orders: {}", self.active_orders.len());
        println!("==================================\n");
    }
    
    /// Cancels all active orders
    async fn cancel_all_orders(&self, cancel_tx: mpsc::Sender<Uuid>) -> Result<()> {
        println!("Cancelling {} active orders", self.active_orders.len());
        
        // Send cancellation requests for all active orders
        for order_id in self.active_orders.keys() {
            if let Err(e) = cancel_tx.send(*order_id).await {
                eprintln!("Error sending cancel request: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Places new orders based on current strategy
    async fn place_new_orders(&mut self, order_tx: mpsc::Sender<CreateOrderRequest>) -> Result<()> {
        // Generate orders from strategy
        let orders = self.strategy.generate_orders(&self.inventory_manager);
        
        println!("Placing {} new orders", orders.len());
        
        // Send orders to order processor
        for order in orders {
            if let Err(e) = order_tx.send(order).await {
                eprintln!("Error sending order request: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Spawns a task to process order placements
    fn spawn_order_processor(
        &self, 
        mut orders: mpsc::Receiver<CreateOrderRequest>,
        fill_tx: mpsc::Sender<(Side, Decimal, Decimal)>
    ) -> tokio::task::JoinHandle<()> {
        let client = self.api_client.clone();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_requests));
        let active_orders = Arc::new(tokio::sync::RwLock::new(self.active_orders.clone()));
        let order_sides = Arc::new(tokio::sync::RwLock::new(self.order_sides.clone()));
        
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
                let order_sides = order_sides.clone();
                let fill_tx = fill_tx.clone();
                let side = order.side;
                
                join_set.spawn(async move {
                    let result = client.place_order(order).await;
                    drop(permit); // Release the permit
                    
                    match result {
                        Ok(order_resp) => {
                            // Check if order was immediately filled
                            if order_resp.status == OrderStatus::Filled || order_resp.status == OrderStatus::PartiallyFilled {
                                // Calculate filled amount
                                let filled_amount = order_resp.filled_base;
                                if !filled_amount.is_zero() {
                                    // Get average fill price
                                    let avg_price = if order_resp.filled_base.is_zero() {
                                        order_resp.limit_price.unwrap_or(Decimal::ZERO)
                                    } else {
                                        order_resp.filled_quote / order_resp.filled_base
                                    };
                                    
                                    // Send fill information to fill processor
                                    if let Err(e) = fill_tx.send((side, filled_amount, avg_price)).await {
                                        eprintln!("Failed to send fill info: {}", e);
                                    }
                                    
                                    println!("[FILL] {} {:.8} @ {:.8}", 
                                             if side == Side::Bid { "BUY" } else { "SELL" },
                                             filled_amount,
                                             avg_price);
                                }
                            }
                            
                            // Store active order information
                            let mut orders = active_orders.write().await;
                            orders.insert(order_resp.id, order_resp.clone());
                            
                            let mut sides = order_sides.write().await;
                            sides.insert(order_resp.id, side);
                            
                            println!("[ORDER] {} {:.8} @ {:?} (ID: {})", 
                                     if side == Side::Bid { "BUY" } else { "SELL" },
                                     order_resp.base_amount,
                                     order_resp.limit_price,
                                     order_resp.id);
                        },
                        Err(e) => {
                            eprintln!("Error placing order: {}", e);
                        }
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
        let order_sides = Arc::new(tokio::sync::RwLock::new(self.order_sides.clone()));
        
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
                let order_sides = order_sides.clone();
                
                join_set.spawn(async move {
                    let result = client.cancel_order(order_id, instrument_id).await;
                    drop(permit);
                    
                    // Remove from tracking regardless of cancel result
                    let mut orders = active_orders.write().await;
                    orders.remove(&order_id);
                    
                    let mut sides = order_sides.write().await;
                    sides.remove(&order_id);
                    
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
    
    /// Spawns a task to process fills
    fn spawn_fill_processor(
        &self,
        mut fills: mpsc::Receiver<(Side, Decimal, Decimal)>
    ) -> tokio::task::JoinHandle<()> {
        let inventory_manager = Arc::new(tokio::sync::Mutex::new(self.inventory_manager.clone()));
        
        tokio::spawn(async move {
            while let Some((side, size, price)) = fills.recv().await {
                let mut inventory = inventory_manager.lock().await;
                inventory.update_position(side, size, price);
            }
        })
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
    
    /// Gets recent trades through the API
    pub async fn get_trades(&self, instrument_id: Uuid, limit: usize) -> Result<Vec<TradeResponse>> {
        let url = format!("{}/instruments/{}/trades", self.base_url, instrument_id);
        
        let resp = self.client.get(&url)
            .query(&[("limit", limit.to_string())])
            .send()
            .await?;
            
        match resp.status() {
            StatusCode::OK => {
                let trades_resp = resp.json::<Vec<TradeResponse>>().await?;
                Ok(trades_resp)
            },
            status => {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                Err(anyhow!("Failed to get trades: {} - {}", status, error_text))
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

/// Helper trait for decimal operations
trait DecimalExt {
    fn sqrt(&self) -> Option<Decimal>;
}

impl DecimalExt for Decimal {
    fn sqrt(&self) -> Option<Decimal> {
        if self.is_sign_negative() {
            return None;
        }
        
        // Convert to f64 for sqrt calculation
        let f = self.to_f64()?;
        let sqrt = f.sqrt();
        
        // Convert back to Decimal using FromPrimitive
        <Decimal as FromPrimitive>::from_f64(sqrt)
    }
}

/// Market making strategy implementation
#[derive(Debug)]
pub struct MarketMakingStrategy {
    /// Configuration for the strategy
    config: MarketMakerConfig,
    
    /// Last known mid price
    last_mid_price: Decimal,
    
    /// Market volatility estimate
    volatility_estimate: Decimal,
    
    /// Recent price history for volatility calculation
    price_history: VecDeque<Decimal>,
    
    /// Random number generator
    rng: rand::rngs::ThreadRng,
    
    /// Timestamp of last volatility update
    last_volatility_update: Instant,
}

impl MarketMakingStrategy {
    /// Creates a new market making strategy
    pub fn new(config: MarketMakerConfig) -> Self {
        Self {
            config: config.clone(),
            last_mid_price: config.base_price,
            volatility_estimate: dec!(0.1), // Initial volatility estimate
            price_history: VecDeque::with_capacity(100),
            rng: thread_rng(),
            last_volatility_update: Instant::now(),
        }
    }
    
    /// Updates the market state based on new market data
    pub fn update_market_state(&mut self, bid_price: Option<Decimal>, ask_price: Option<Decimal>, trades: &[TradeResponse]) {
        // Update mid price if we have both bid and ask
        if let (Some(bid), Some(ask)) = (bid_price, ask_price) {
            let mid_price = (bid + ask) / Decimal::from(2);
            self.update_price_history(mid_price);
            self.last_mid_price = mid_price;
        }
        // If we only have bid
        else if let Some(bid) = bid_price {
            self.update_price_history(bid);
            self.last_mid_price = bid;
        }
        // If we only have ask
        else if let Some(ask) = ask_price {
            self.update_price_history(ask);
            self.last_mid_price = ask;
        }
        
        // Update volatility if needed
        if self.last_volatility_update.elapsed() > Duration::from_secs(10) {
            self.update_volatility();
            self.last_volatility_update = Instant::now();
        }
        
        // Incorporate trade information into volatility estimation
        if !trades.is_empty() {
            // Use trade prices to refine volatility estimate
            let trade_prices: Vec<Decimal> = trades.iter().map(|t| t.price).collect();
            if !trade_prices.is_empty() {
                let min_price = trade_prices.iter().min().unwrap();
                let max_price = trade_prices.iter().max().unwrap();
                
                if *min_price > Decimal::ZERO && *max_price > Decimal::ZERO {
                    let range_pct = (*max_price - *min_price) / self.last_mid_price * Decimal::from(100);
                    
                    // Blend with current volatility
                    self.volatility_estimate = self.volatility_estimate * dec!(0.9) + range_pct * dec!(0.1);
                }
            }
        }
    }
    
    /// Updates the price history for volatility calculation
    fn update_price_history(&mut self, price: Decimal) {
        self.price_history.push_back(price);
        
        // Keep history bounded
        while self.price_history.len() > 100 {
            self.price_history.pop_front();
        }
    }
    
    /// Updates the volatility estimate
    fn update_volatility(&mut self) {
        if self.price_history.len() < 2 {
            return;
        }
        
        // Calculate returns
        let mut returns = Vec::with_capacity(self.price_history.len() - 1);
        let mut prev_price = self.price_history[0];
        
        for price in self.price_history.iter().skip(1) {
            let return_pct = (*price - prev_price) / prev_price * Decimal::from(100);
            returns.push(return_pct);
            prev_price = *price;
        }
        
        // Calculate standard deviation of returns
        if returns.is_empty() {
            return;
        }
        
        let mean = returns.iter().sum::<Decimal>() / Decimal::from(returns.len());
        let variance: Decimal = returns.iter()
            .map(|r| {
                let diff = *r - mean;
                diff * diff  // Square the difference instead of using pow
            })
            .sum::<Decimal>() / Decimal::from(returns.len());
        
        let std_dev = variance.sqrt().unwrap_or(dec!(0.1));
        
        // Update volatility estimate with some smoothing
        self.volatility_estimate = self.volatility_estimate * dec!(0.7) + std_dev * dec!(0.3);
    }
    
    /// Calculates the bid-ask spread based on market conditions
    fn calculate_spread(&self) -> Decimal {
        // Base spread from config
        let mut spread = self.config.base_spread_pct;
        
        // Adjust based on volatility
        spread += self.volatility_estimate * self.config.volatility_factor / Decimal::from(100);
        
        // Ensure minimum spread
        spread = spread.max(dec!(0.01));
        
        spread
    }
    
    /// Calculates bid and ask prices with inventory skew adjustment
    pub fn calculate_bid_ask_prices(&self, inventory_skew: Decimal) -> (Decimal, Decimal) {
        // Get the spread
        let spread_pct = self.calculate_spread();
        let half_spread_pct = spread_pct / Decimal::from(2);
        
        // Apply inventory skew (negative skew = lower prices, positive skew = higher prices)
        let skew_adjustment = self.last_mid_price * inventory_skew;
        
        // Calculate bid and ask prices
        let mid_price_adjusted = self.last_mid_price + skew_adjustment;
        let bid_price = mid_price_adjusted * (Decimal::ONE - half_spread_pct / Decimal::from(100));
        let ask_price = mid_price_adjusted * (Decimal::ONE + half_spread_pct / Decimal::from(100));
        
        (bid_price, ask_price)
    }
    
    /// Determines the optimal order size based on market conditions and inventory
    pub fn calculate_order_size(&mut self, side: Side, base_position: Decimal) -> Decimal {
        let min_size = self.config.min_order_size;
        let max_size = self.config.max_order_size;
        
        // Base size is randomly selected between min and max
        let base_size: Decimal = <Decimal as FromPrimitive>::from_f64(
            self.rng.gen_range(
                num_traits::ToPrimitive::to_f64(&min_size).unwrap_or(0.01)..num_traits::ToPrimitive::to_f64(&max_size).unwrap_or(1.0)
            )
        ).unwrap_or(min_size);
        
        // Adjust size based on inventory - reduce size on the side that would increase imbalance
        let position_adjustment = if base_position.is_zero() {
            Decimal::ONE
        } else if (side == Side::Bid && base_position > Decimal::ZERO) ||
                  (side == Side::Ask && base_position < Decimal::ZERO) {
            // Reducing size when adding to existing imbalance
            let abs_position = base_position.abs();
            let factor = Decimal::ONE - (abs_position / max_size).min(dec!(0.9));
            factor
        } else {
            // Increasing size when reducing imbalance
            let abs_position = base_position.abs();
            let factor = Decimal::ONE + (abs_position / max_size).min(dec!(0.5));
            factor
        };
        
        // Apply the adjustment
        let adjusted_size = base_size * position_adjustment;
        
        // Ensure size is within limits
        adjusted_size.max(min_size).min(max_size)
    }
    
    /// Generates orders for market making
    pub fn generate_orders(&mut self, inventory_manager: &InventoryManager) -> Vec<CreateOrderRequest> {
        // Calculate inventory skew
        let inventory_skew = inventory_manager.calculate_price_skew(self.last_mid_price);
        
        // Calculate bid and ask prices
        let (base_bid_price, base_ask_price) = self.calculate_bid_ask_prices(inventory_skew);
        
        // Determine number of levels
        let levels = self.config.price_levels;
        
        // Determine total orders to generate
        let total_orders = levels * 2; // bid and ask at each level
        let mut orders = Vec::with_capacity(total_orders);
        
        // Generate bid orders
        for level in 0..levels {
            // Calculate level price (each level is slightly lower)
            let level_factor = Decimal::from(level) * dec!(0.05) / Decimal::from(100);
            let bid_price = base_bid_price * (Decimal::ONE - level_factor);
            
            // Calculate order size
            let size = self.calculate_order_size(Side::Bid, inventory_manager.base_position);
            
            orders.push(CreateOrderRequest {
                ext_id: Some(format!("mm-{}", Uuid::new_v4())),
                account_id: self.config.account_id,
                order_type: OrderType::Limit,
                instrument_id: self.config.instrument_id.expect("Instrument ID must be set"),
                side: Side::Bid,
                limit_price: Some(bid_price),
                trigger_price: None,
                base_amount: size,
                time_in_force: TimeInForce::GTC,
            });
        }
        
        // Generate ask orders
        for level in 0..levels {
            // Calculate level price (each level is slightly higher)
            let level_factor = Decimal::from(level) * dec!(0.05) / Decimal::from(100);
            let ask_price = base_ask_price * (Decimal::ONE + level_factor);
            
            // Calculate order size
            let size = self.calculate_order_size(Side::Ask, inventory_manager.base_position);
            
            orders.push(CreateOrderRequest {
                ext_id: Some(format!("mm-{}", Uuid::new_v4())),
                account_id: self.config.account_id,
                order_type: OrderType::Limit,
                instrument_id: self.config.instrument_id.expect("Instrument ID must be set"),
                side: Side::Ask,
                limit_price: Some(ask_price),
                trigger_price: None,
                base_amount: size,
                time_in_force: TimeInForce::GTC,
            });
        }
        
        orders
    }
}

/// Inventory manager for tracking positions and calculating inventory-based adjustments
#[derive(Debug, Clone)]
pub struct InventoryManager {
    /// Base currency position (e.g., BTC)
    base_position: Decimal,
    
    /// Quote currency position (e.g., USD)
    quote_position: Decimal,
    
    /// Initial capital in quote currency
    initial_capital: Decimal,
    
    /// Maximum allowed imbalance as percentage of capital
    max_imbalance_pct: Decimal,
    
    /// Inventory skew factor (0.0-1.0)
    inventory_skew_factor: Decimal,
    
    /// Recent trades for calculating realized PnL
    recent_trades: VecDeque<(Side, Decimal, Decimal)>, // (side, size, price)
    
    /// Maximum trades to track for PnL calculation
    max_trade_history: usize,
}

impl InventoryManager {
    /// Creates a new inventory manager
    pub fn new(capital: Decimal, max_imbalance_pct: Decimal) -> Self {
        Self {
            base_position: Decimal::ZERO,
            quote_position: Decimal::ZERO,
            initial_capital: capital,
            max_imbalance_pct,
            inventory_skew_factor: dec!(0.2),
            recent_trades: VecDeque::new(),
            max_trade_history: 100,
        }
    }
    
    /// Updates the current position
    pub fn update_position(&mut self, side: Side, size: Decimal, price: Decimal) {
        match side {
            Side::Bid => {
                self.base_position += size;
                self.quote_position -= size * price;
            },
            Side::Ask => {
                self.base_position -= size;
                self.quote_position += size * price;
            }
        }
        
        // Track trade for PnL calculation
        self.recent_trades.push_back((side, size, price));
        if self.recent_trades.len() > self.max_trade_history {
            self.recent_trades.pop_front();
        }
    }
    
    /// Returns the current quote position
    pub fn quote_position(&self) -> Decimal {
        self.quote_position
    }
    
    /// Calculates the total value of the inventory
    pub fn total_value(&self, mid_price: Decimal) -> Decimal {
        self.quote_position + (self.base_position * mid_price)
    }
    
    /// Calculates realized PnL
    pub fn realized_pnl(&self) -> Decimal {
        self.total_value(Decimal::ZERO) - self.initial_capital
    }
    
    /// Calculates the price skew based on current inventory
    pub fn calculate_price_skew(&self, mid_price: Decimal) -> Decimal {
        let max_position = self.initial_capital / mid_price;
        let position_ratio = self.base_position / max_position;
        position_ratio * dec!(0.1) // Adjust prices by up to 10% based on inventory
    }
}

/// Data transfer objects
mod dto {
    use serde::{Serialize, Deserialize};
    use chrono::{DateTime, Utc};
    use rust_decimal::Decimal;
    use uuid::Uuid;
    use ultimate_matching::types::{Side, OrderType, TimeInForce};

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
        pub status: String,
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

    /// Trade response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TradeResponse {
        /// Unique identifier for the trade
        pub id: Uuid,
        /// Identifier for the instrument traded
        pub instrument_id: Uuid,
        /// ID of the maker order
        pub maker_order_id: Uuid,
        /// ID of the taker order
        pub taker_order_id: Uuid,
        /// Price at which the trade occurred
        pub price: Decimal,
        /// Quantity traded in base units
        pub base_amount: Decimal,
        /// Quantity traded in quote units
        pub quote_amount: Decimal,
        /// Timestamp when the trade occurred
        pub created_at: DateTime<Utc>,
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

/// Represents an order book that maintains bid and ask orders
#[derive(Debug, Clone)]
pub struct OrderBook {
    /// Map of bid prices to their volumes
    bids: BTreeMap<Decimal, Decimal>,
    /// Map of ask prices to their volumes
    asks: BTreeMap<Decimal, Decimal>,
    /// Timestamp of last update
    last_update: Instant,
}

impl OrderBook {
    /// Creates a new order book
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update: Instant::now(),
        }
    }
    
    /// Updates the order book with new orders
    pub fn update(&mut self, orders: Vec<Order>) {
        self.bids.clear();
        self.asks.clear();
        
        for order in orders {
            if let Some(price) = order.limit_price {
                match order.side {
                    Side::Bid => {
                        self.bids.insert(price, order.remaining_base);
                    },
                    Side::Ask => {
                        self.asks.insert(price, order.remaining_base);
                    }
                }
            }
        }
        
        self.last_update = Instant::now();
    }
    
    /// Returns the best bid price
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.keys().next_back().copied()
    }
    
    /// Returns the best ask price
    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }
    
    /// Returns the mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / dec!(2)),
            _ => None
        }
    }
    
    /// Returns the spread
    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None
        }
    }
    
    /// Returns the volume at a given price level
    pub fn volume_at_price(&self, price: Decimal, side: Side) -> Decimal {
        match side {
            Side::Bid => self.bids.get(&price).copied().unwrap_or(Decimal::ZERO),
            Side::Ask => self.asks.get(&price).copied().unwrap_or(Decimal::ZERO)
        }
    }
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Validate arguments
    if args.workers == 0 {
        return Err(anyhow!("Workers must be greater than 0"));
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
    println!("Neutral Market Maker starting up...");
    println!("API URL: {}", args.api_url);
    println!("Spread: {}%", args.spread);
    println!("Capital: {} quote units", args.capital);
    println!("Max Imbalance: {}%", args.max_imbalance);
    
    // Initialize API client
    let api_client = ApiClient::new(&args.api_url);
    
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
    
    // Create account ID (in a real system, this would be a real account)
    let account_id = Uuid::new_v4();
    
    // Create market maker config
    let mut config = MarketMakerConfig::new(instrument_id, account_id);
    
    // Update with command line parameters
    config.base_spread_pct = <Decimal as FromPrimitive>::from_f64(args.spread).unwrap_or(dec!(0.1));
    config.capital = <Decimal as FromPrimitive>::from_f64(args.capital).unwrap_or(dec!(100000.0));
    config.max_imbalance_pct = <Decimal as FromPrimitive>::from_f64(args.max_imbalance).unwrap_or(dec!(5.0));
    
    // Save the config
    let config_json = serde_json::to_string_pretty(&config)?;
    std::fs::write(&args.config, config_json)?;
    println!("Saved configuration to {}", args.config);
    
    // Create and run the market maker
    let mut market_maker = NeutralMarketMaker::new(config, &args.api_url);
    
    // Run the market maker
    market_maker.run().await?;
    
    Ok(())
} 
//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This is the main entry point for the API server.
// It sets up the event system, creates the API server, and starts listening for requests.
//--------------------------------------------------------------------------------------------------

use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use std::io::{self, Write};
use tracing::{info, Level};
use tokio::sync::RwLock;
use tokio::time::sleep;
use chrono::{DateTime, Utc};

use ultimate_matching::{
    Api,
    events::{EventBus, EventDispatcher, EventLogger, PersistenceEventHandler, EventHandler, MatchingEngineEvent, EventResult},
    types::{Order, Trade, Side},
};

/// Custom handler to track and display market data
struct MarketDisplayHandler {
    /// Order tracker
    orders: Arc<RwLock<HashMap<uuid::Uuid, Order>>>,
    /// Trade tracker
    trades: Arc<RwLock<Vec<Trade>>>,
    /// Best bid and ask by instrument
    quotes: Arc<RwLock<HashMap<uuid::Uuid, (Option<rust_decimal::Decimal>, Option<rust_decimal::Decimal>)>>>,
}

impl MarketDisplayHandler {
    fn new() -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
            trades: Arc::new(RwLock::new(Vec::with_capacity(100))),
            quotes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn print_order(&self, order: &Order) {
        println!("[ORDER] {} {} {} @ {:?} (ID: {})", 
            match order.side {
                Side::Bid => "BUY ",
                Side::Ask => "SELL",
            },
            order.base_amount,
            order.instrument_id,
            order.limit_price,
            order.id
        );
    }
    
    async fn print_trade(&self, trade: &Trade) {
        println!("[TRADE] {} @ {} (Maker: {}, Taker: {})", 
            trade.base_amount,
            trade.price,
            trade.maker_order_id,
            trade.taker_order_id
        );
    }
    
    async fn start_display_task(&self) -> tokio::task::JoinHandle<()> {
        let orders = self.orders.clone();
        let trades = self.trades.clone();
        let quotes = self.quotes.clone();
        
        tokio::spawn(async move {
            loop {
                // Clear screen and move cursor to home position
                print!("\x1B[2J\x1B[H");
                io::stdout().flush().unwrap();
                
                // Display market data as ASCII table
                println!("╔══════════════════════════════════════════════════════════════╗");
                println!("║                   MARKET DATA DISPLAY                        ║");
                println!("╠═════════════════╦════════════════╦════════════════╦═════════╣");
                println!("║    INSTRUMENT   ║      BID       ║      ASK       ║  SPREAD ║");
                println!("╠═════════════════╬════════════════╬════════════════╬═════════╣");
                
                // Print quotes
                let quotes_data = quotes.read().await;
                for (instrument_id, (bid, ask)) in quotes_data.iter() {
                    let bid_str = bid.map_or("-".to_string(), |b| format!("{:.2}", b));
                    let ask_str = ask.map_or("-".to_string(), |a| format!("{:.2}", a));
                    let spread = match (bid, ask) {
                        (Some(b), Some(a)) => format!("{:.2}", a - b),
                        _ => "-".to_string(),
                    };
                    
                    println!("║ {:.8}... ║ {:^14} ║ {:^14} ║ {:^7} ║", 
                        instrument_id.to_string(), bid_str, ask_str, spread);
                }
                
                println!("╠═════════════════╩════════════════╩════════════════╩═════════╣");
                println!("║                   RECENT TRADES                              ║");
                println!("╠═══════════════════════╦════════════════╦════════════════════╣");
                println!("║         TIME          ║     PRICE      ║       AMOUNT       ║");
                println!("╠═══════════════════════╬════════════════╬════════════════════╣");
                
                // Print recent trades (last 5)
                let trades_data = trades.read().await;
                let trade_count = trades_data.len();
                let start_idx = if trade_count > 5 { trade_count - 5 } else { 0 };
                
                for trade in trades_data.iter().skip(start_idx) {
                    println!("║ {} ║ {:^14} ║ {:^18} ║", 
                        format_time(&trade.created_at), 
                        format!("{:.2}", trade.price),
                        format!("{:.4}", trade.base_amount));
                }
                
                // Fill empty rows if needed
                for _ in 0..(5 - trade_count.min(5)) {
                    println!("║                       ║                ║                    ║");
                }
                
                println!("╠═══════════════════════╩════════════════╩════════════════════╣");
                println!("║ Total Orders: {:<6}                  Total Trades: {:<6} ║", 
                    orders.read().await.len(),
                    trades_data.len());
                println!("╚══════════════════════════════════════════════════════════════╝");
                
                // Sleep for 5 seconds
                sleep(Duration::from_secs(5)).await;
            }
        })
    }
}

fn format_time(time: &DateTime<Utc>) -> String {
    time.format("%H:%M:%S.%3f").to_string()
}

#[async_trait::async_trait]
impl EventHandler for MarketDisplayHandler {
    fn event_types(&self) -> Vec<&'static str> {
        vec![
            "OrderAdded", 
            "OrderMatched", 
            "OrderCancelled", 
            "OrderStatusChanged",
            "TradeExecuted", 
            "DepthUpdated"
        ]
    }
    
    async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()> {
        match &event {
            MatchingEngineEvent::OrderAdded { order, .. } => {
                // Print and store the order
                self.print_order(order).await;
                self.orders.write().await.insert(order.id, order.clone());
            },
            MatchingEngineEvent::TradeExecuted { trade, .. } => {
                // Print and store the trade
                self.print_trade(trade).await;
                self.trades.write().await.push(trade.clone());
            },
            MatchingEngineEvent::DepthUpdated { depth, .. } => {
                // Update bid/ask quotes
                let mut quotes = self.quotes.write().await;
                quotes.insert(
                    depth.instrument_id,
                    (
                        depth.best_bid().map(|p| rust_decimal::Decimal::from(p) / rust_decimal::Decimal::from(100_000)),
                        depth.best_ask().map(|p| rust_decimal::Decimal::from(p) / rust_decimal::Decimal::from(100_000))
                    )
                );
            },
            MatchingEngineEvent::OrderCancelled { order, .. } => {
                println!("[CANCEL] Order cancelled: {}", order.id);
                self.orders.write().await.remove(&order.id);
            },
            _ => {}
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("Starting matching engine API server");
    
    // Set up the event system
    let event_bus = EventBus::default();
    
    // Create a logger handler with buffer size
    let logger = Arc::new(EventLogger::new(1000)); // Buffer size of 1000 events
    
    // Create a market display handler
    let display_handler = Arc::new(MarketDisplayHandler::new());
    
    // Start the display task
    let _display_task = display_handler.start_display_task().await;
    
    // Create a persistence handler if possible
    let persistence_handler = match PersistenceEventHandler::new(
        std::path::Path::new("./events"), 
        1000
    ) {
        Ok(handler) => {
            info!("Persistence handler created. Events will be stored in ./events");
            Some(Arc::new(handler))
        },
        Err(e) => {
            tracing::error!("Failed to create persistence handler: {}", e);
            None
        }
    };
    
    // Register handlers with the dispatcher
    let dispatcher = EventDispatcher::new(event_bus.clone());
    dispatcher.register_handler(logger).await;
    dispatcher.register_handler(display_handler).await;
    
    if let Some(handler) = persistence_handler {
        dispatcher.register_handler(handler).await;
    }
    
    // Start the event dispatcher
    let _handle = dispatcher.start().await;
    
    // Create the API server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    info!("API server listening on {}", addr);
    
    // Create and serve the API
    let api = Api::new(addr, event_bus);
    api.serve().await?;
    
    Ok(())
} 
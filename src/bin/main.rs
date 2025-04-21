//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This is the main entry point for the Ultimate Matching Engine system.
// It sets up the event system, initializes matching engines for instruments,
// and optionally starts the API server.
//--------------------------------------------------------------------------------------------------
// To run a demo: cargo run --bin main -- --demo
// To run the API server: cargo run --bin main -- --api
// To run both: cargo run --bin main -- --demo --api
// Advanced: cargo run --bin main -- --demo --api --port 3001
// cargo run --bin main -- --api --port 8080 --event-dir ./my-events --buffer-size 2000 
/// This lets you: Set a custom port for the API server
/// Set a custom directory for event persistence
/// Configure the event buffer size
use std::net::SocketAddr;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::signal;
use tracing::{info, Level};
use uuid::Uuid;
use std::path::PathBuf;
use async_trait::async_trait;

use ultimate_matching::{
    Api,
    api::AppState,
    matching_engine::MatchingEngine,
    events::{EventBus, EventDispatcher, EventLogger, PersistenceEventHandler, EventHandler, MatchingEngineEvent, EventResult},
    types::{Order, Side, OrderType, OrderStatus, TimeInForce},
};

/// Console handler that displays order events in the terminal
struct OrderConsoleHandler;

#[async_trait]
impl EventHandler for OrderConsoleHandler {
    fn event_types(&self) -> Vec<&'static str> {
        vec![
            "OrderAdded",
            "OrderMatched",
            "OrderCancelled",
            "TradeExecuted"
        ]
    }
    
    async fn handle_event(&self, event: MatchingEngineEvent) -> EventResult<()> {
        match &event {
            MatchingEngineEvent::OrderAdded { order, timestamp } => {
                info!("📝 ORDER ADDED [{}]:", timestamp);
                info!("  ID: {}", order.id);
                if let Some(ext_id) = &order.ext_id {
                    info!("  External ID: {}", ext_id);
                }
                info!("  Type: {:?} {:?}", order.side, order.order_type);
                if let Some(price) = order.limit_price {
                    info!("  Price: ${}", price);
                }
                info!("  Quantity: {} BTC", order.base_amount);
                info!("  Account: {}", order.account_id);
                info!("  Instrument: {}", order.instrument_id);
                info!("  Status: {:?}", order.status);
                info!("------------------------------------");
            },
            MatchingEngineEvent::OrderMatched { order, matched_quantity, timestamp } => {
                info!("✅ ORDER MATCHED [{}]:", timestamp);
                info!("  ID: {}", order.id);
                info!("  Matched Quantity: {} BTC", matched_quantity);
                info!("  Remaining Quantity: {} BTC", order.remaining_base);
                info!("  Status: {:?}", order.status);
                info!("------------------------------------");
            },
            MatchingEngineEvent::OrderCancelled { order, timestamp } => {
                info!("❌ ORDER CANCELLED [{}]:", timestamp);
                info!("  ID: {}", order.id);
                if let Some(ext_id) = &order.ext_id {
                    info!("  External ID: {}", ext_id);
                }
                info!("  Status: {:?}", order.status);
                info!("------------------------------------");
            },
            MatchingEngineEvent::TradeExecuted { trade, timestamp } => {
                info!("🔄 TRADE EXECUTED [{}]:", timestamp);
                info!("  ID: {}", trade.id);
                info!("  Maker Order: {}", trade.maker_order_id);
                info!("  Taker Order: {}", trade.taker_order_id);
                info!("  Price: ${}", trade.price);
                info!("  Quantity: {} BTC", trade.base_amount);
                info!("  Value: ${}", trade.quote_amount);
                info!("  Instrument: {}", trade.instrument_id);
                info!("------------------------------------");
            },
            _ => {}
        }
        Ok(())
    }
}

/// CLI options for the application
#[derive(StructOpt, Debug)]
#[structopt(name = "ultimate-matching", about = "Ultimate Matching Engine")]
struct Opt {
    /// Whether to start the API server
    #[structopt(long, help = "Start the API server")]
    api: bool,

    /// Port to use for the API server
    #[structopt(long, default_value = "3000", help = "API server port")]
    port: u16,

    /// Directory to store event logs
    #[structopt(long, parse(from_os_str), default_value = "./events", help = "Directory to store event logs")]
    event_dir: PathBuf,

    /// Event buffer size
    #[structopt(long, default_value = "1000", help = "Event buffer size")]
    buffer_size: usize,

    /// Whether to create a test instrument and add sample orders
    #[structopt(long, help = "Create a test instrument and add sample orders")]
    demo: bool,
}

/// Helper function to create a test order
fn create_test_order(side: Side, price: f64, quantity: f64, instrument_id: Uuid) -> Order {
    use chrono::Utc;

    // Convert to integer values with 6 decimal places of precision
    let price_i64 = (price * 1_000_000.0) as i64;
    let quantity_u64 = (quantity * 1_000_000.0) as u64;
    let quote_amount = ((price * quantity) * 1_000_000.0) as u64;
    
    let now = Utc::now();
    
    Order {
        id: Uuid::new_v4(),
        ext_id: Some("test-order".to_string()),
        account_id: Uuid::new_v4(),
        order_type: OrderType::Limit,
        instrument_id,
        side,
        limit_price: Some(price_i64),
        trigger_price: None,
        base_amount: quantity_u64,
        remaining_base: quantity_u64,
        filled_quote: 0,
        filled_base: 0,
        remaining_quote: quote_amount,
        expiration_date: now + chrono::Duration::days(365),
        status: OrderStatus::Submitted,
        created_at: now,
        updated_at: now,
        trigger_by: None,
        created_from: ultimate_matching::types::CreatedFrom::Api,
        sequence_id: 1,
        time_in_force: TimeInForce::GTC,
    }
}

/// Starts demo matching engine with sample orders
async fn run_demo(event_bus: &EventBus) -> Uuid {
    // Create a shared AppState for the instrument
    let app_state = Arc::new(AppState::new(event_bus.clone()));
    
    // Create BTC-USD instrument
    let instrument_id = create_btc_usd_instrument(&app_state).await;
    info!("Created BTC-USD instrument with ID: {}", instrument_id);

    // Create the matching engine with events
    let mut engine = MatchingEngine::with_event_bus(instrument_id, event_bus.clone());
    
    info!("Adding sample orders for BTC-USD...");
    
    // Add a sell order at 100
    let sell_order = create_test_order(Side::Ask, 100.0, 1.0, instrument_id);
    let result = engine.process_order(sell_order, TimeInForce::GTC).unwrap();
    info!("Added sell order: {:?}", result.processed_order.unwrap().id);
    
    // Add a buy order at 99 (won't match)
    let buy_order1 = create_test_order(Side::Bid, 99.0, 1.0, instrument_id);
    let result = engine.process_order(buy_order1, TimeInForce::GTC).unwrap();
    info!("Added buy order (won't match): {:?}", result.processed_order.unwrap().id);
    
    // Add a buy order at 100 (will match)
    let buy_order2 = create_test_order(Side::Bid, 100.0, 0.5, instrument_id);
    let result = engine.process_order(buy_order2, TimeInForce::GTC).unwrap();
    info!("Added matching buy order: {:?}", result.processed_order.unwrap().id);
    
    if !result.trades.is_empty() {
        info!("Generated {} trades!", result.trades.len());
        for trade in result.trades {
            info!("Trade: {} BTC @ ${}", trade.base_amount, trade.price);
        }
    }
    
    // Check the order book depth
    let depth = engine.get_depth(10);
    info!("Current BTC-USD orderbook depth:");
    if !depth.bids.is_empty() {
        info!("Bids:");
        for level in depth.bids {
            info!("  {} BTC @ ${}", level.volume, level.price);
        }
    }
    if !depth.asks.is_empty() {
        info!("Asks:");
        for level in depth.asks {
            info!("  {} BTC @ ${}", level.volume, level.price);
        }
    }
    
    instrument_id
}

/// Creates the BTC-USD instrument and returns its ID
async fn create_btc_usd_instrument(app_state: &Arc<AppState>) -> Uuid {
    // Generate a stable ID for BTC-USD
    let instrument_id = Uuid::new_v4();
    
    // Register the instrument in the AppState
    app_state.add_instrument(instrument_id).await;
    
    // Log information about the instrument
    info!("BTC-USD instrument created with ID: {}", instrument_id);
    
    instrument_id
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let opt = Opt::from_args();
    
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    info!("Starting Ultimate Matching Engine");
    
    // Set up the event system
    let event_bus = EventBus::default();
    
    // Create a logger handler with buffer size
    let logger = Arc::new(EventLogger::new(opt.buffer_size));
    
    // Create a console handler for orders
    let order_console = Arc::new(OrderConsoleHandler);
    
    // Create a persistence handler if possible
    let persistence_handler = match PersistenceEventHandler::new(
        &opt.event_dir, 
        opt.buffer_size
    ) {
        Ok(handler) => {
            info!("Persistence handler created. Events will be stored in {}", opt.event_dir.display());
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
    dispatcher.register_handler(order_console).await;
    
    if let Some(handler) = persistence_handler {
        dispatcher.register_handler(handler).await;
    }
    
    // Start the event dispatcher
    let _handle = dispatcher.start().await;
    
    // Create test instrument and demo orders if requested
    if opt.demo {
        let instrument_id = run_demo(&event_bus).await;
        info!("Demo completed with instrument ID: {}", instrument_id);
    }
    
    // Start the API server if requested
    if opt.api {
        // Create the API server
        let addr = SocketAddr::from(([127, 0, 0, 1], opt.port));
        let api = Api::new(addr, event_bus);
        
        // Start the API server in a separate task
        info!("Starting API server on http://127.0.0.1:{}", opt.port);
        tokio::spawn(async move {
            if let Err(e) = api.serve().await {
                tracing::error!("API server error: {}", e);
            }
        });
        
        info!("API server started. Press Ctrl+C to stop.");
        
        // Wait for Ctrl+C signal
        signal::ctrl_c().await?;
        info!("Shutdown signal received, stopping...");
    } else {
        // If API is not enabled and demo mode is done, just exit
        if opt.demo {
            info!("Demo mode completed. Use --api to start the API server.");
        } else {
            info!("No actions specified. Use --demo to run a demo or --api to start the API server.");
        }
    }
    
    Ok(())
} 
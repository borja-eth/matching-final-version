mod types;

use std::sync::Arc;
use chrono::Utc;
use rust_decimal_macros::dec;
use uuid::Uuid;
use rust_decimal::Decimal;
use num_traits::FromPrimitive;
use tracing_subscriber;

use ultimate_matching::{
    OrderType, Order, Side, OrderStatus, TimeInForce, MatchingEngine,
    events::{EventBus, EventDispatcher, EventHandler, MatchingEngineEvent, EventResult, PersistenceEventHandler}
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
    
    // Convert f64 to Decimal correctly using FromPrimitive
    let price_dec = Decimal::from_f64(price).unwrap_or_else(|| dec!(0));
    let qty_dec = Decimal::from_f64(qty).unwrap_or_else(|| dec!(0));
    
    Order {
        id: Uuid::new_v4(),
        ext_id: Some("example-order".to_string()),
        account_id: Uuid::new_v4(),
        order_type: OrderType::Limit,
        instrument_id,
        side,
        limit_price: Some(price_dec),
        trigger_price: None,
        base_amount: qty_dec,
        remaining_base: qty_dec,
        filled_quote: dec!(0),
        filled_base: dec!(0),
        remaining_quote: price_dec * qty_dec,
        expiration_date: now + chrono::Duration::days(7),
        status: OrderStatus::New,
        created_at: now,
        updated_at: now,
        trigger_by: None,
        created_from: ultimate_matching::types::CreatedFrom::Api,
        sequence_id: 0,
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing (for logging)
    tracing_subscriber::fmt::init();
    
    println!("Starting event-driven matching engine example");
    
    // Create an instrument ID
    let instrument_id = Uuid::new_v4();
    println!("Instrument ID: {}", instrument_id);
    
    // Set up the event system
    let event_bus = EventBus::default();
    
    // Create handlers
    let console_handler = Arc::new(ConsoleEventHandler);
    
    // Create a persistence handler to store events
    let persistence_handler = match PersistenceEventHandler::new(std::path::Path::new("./events"), 1000) {
        Ok(handler) => {
            println!("Persistence handler created. Events will be stored in ./events");
            Some(Arc::new(handler))
        },
        Err(e) => {
            eprintln!("Failed to create persistence handler: {}", e);
            None
        }
    };
    
    // Register handlers with the dispatcher
    let dispatcher = EventDispatcher::new(event_bus.clone());
    dispatcher.register_handler(console_handler).await;
    
    if let Some(handler) = persistence_handler.clone() {
        dispatcher.register_handler(handler).await;
    }
    
    let _handle = dispatcher.start().await;
    
    // Create the matching engine with events
    let mut engine = MatchingEngine::with_event_bus(instrument_id, event_bus);
    
    // Add some orders and see the events
    println!("\nAdding orders...");
    
    // Add a sell order at 100
    let sell_order = create_test_order(Side::Ask, 100.0, 1.0, instrument_id);
    engine.process_order(sell_order, TimeInForce::GTC).unwrap();
    
    // Add a buy order at 99 (won't match)
    let buy_order1 = create_test_order(Side::Bid, 99.0, 1.0, instrument_id);
    engine.process_order(buy_order1, TimeInForce::GTC).unwrap();
    
    // Add a buy order at 100 (will match)
    let buy_order2 = create_test_order(Side::Bid, 100.0, 0.5, instrument_id);
    engine.process_order(buy_order2, TimeInForce::GTC).unwrap();
    
    // Get depth
    let depth = engine.get_depth(10);
    println!("\nCurrent depth:");
    println!("Best bid: {:?}", depth.best_bid());
    println!("Best ask: {:?}", depth.best_ask());
    println!("Spread: {:?}", depth.spread());
    
    // Allow events to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\nExample completed!");
    
    if let Some(_) = persistence_handler {
        println!("Events have been persisted to the ./events directory");
    }
}

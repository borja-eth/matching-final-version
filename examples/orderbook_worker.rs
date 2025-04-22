use uuid::Uuid;
use chrono::Utc;

use ultimate_matching::domain::models::types::{
    Order, Side, OrderType, OrderStatus, TimeInForce, CreatedFrom
};
use ultimate_matching::domain::services::orderbook::OrderBookWorker;

/// Creates a test order for the specified side.
fn create_test_order(side: Side, price: i64, quantity: u64, instrument_id: Uuid) -> Order {
    let now = Utc::now();
    Order {
        id: Uuid::new_v4(),
        ext_id: Some("test-order".to_string()),
        account_id: Uuid::new_v4(),
        order_type: OrderType::Limit,
        instrument_id,
        side,
        limit_price: Some(price),
        trigger_price: None,
        base_amount: quantity,
        remaining_base: quantity,
        filled_quote: 0,
        filled_base: 0,
        remaining_quote: price as u64 * quantity,
        expiration_date: now + chrono::Duration::days(365),
        status: OrderStatus::Submitted,
        created_at: now,
        updated_at: now,
        trigger_by: None,
        created_from: CreatedFrom::Api,
        sequence_id: 1,
        time_in_force: TimeInForce::GTC,
    }
}

#[tokio::main]
async fn main() {
    // Create a new instrument ID
    let instrument_id = Uuid::new_v4();
    println!("Starting OrderBookWorker for instrument {}", instrument_id);
    
    // Create and start the worker
    let worker = OrderBookWorker::new(instrument_id);
    let (client, _handle) = worker.start();
    
    // Create some orders
    let bid_order1 = create_test_order(Side::Bid, 100_000, 5_000, instrument_id);
    let bid_order2 = create_test_order(Side::Bid, 99_000, 10_000, instrument_id);
    let ask_order1 = create_test_order(Side::Ask, 101_000, 3_000, instrument_id);
    let ask_order2 = create_test_order(Side::Ask, 102_000, 7_000, instrument_id);
    
    // Add orders to the book
    println!("Adding orders to the book...");
    client.add_order(bid_order1.clone()).await.expect("Failed to add bid order 1");
    client.add_order(bid_order2.clone()).await.expect("Failed to add bid order 2");
    client.add_order(ask_order1.clone()).await.expect("Failed to add ask order 1");
    client.add_order(ask_order2.clone()).await.expect("Failed to add ask order 2");
    
    // Get the current depth
    let depth = client.get_depth(10).await.expect("Failed to get depth");
    println!("Current depth:");
    println!("Bids:");
    for bid in &depth.bids {
        println!("  Price: {}, Volume: {}, Orders: {}", 
            bid.price,
            bid.volume,
            bid.order_count);
    }
    println!("Asks:");
    for ask in &depth.asks {
        println!("  Price: {}, Volume: {}, Orders: {}", 
            ask.price,
            ask.volume,
            ask.order_count);
    }
    
    // Get best bid and ask
    let best_bid = client.get_best_bid().await.expect("Failed to get best bid");
    let best_ask = client.get_best_ask().await.expect("Failed to get best ask");
    
    println!("Best bid: {:?}", best_bid.map(|o| (o.limit_price.unwrap(), o.remaining_base)));
    println!("Best ask: {:?}", best_ask.map(|o| (o.limit_price.unwrap(), o.remaining_base)));
    
    // Remove an order
    println!("Removing bid order 1...");
    client.remove_order(bid_order1.id).await.expect("Failed to remove order");
    
    // Get the updated depth
    let depth = client.get_depth(10).await.expect("Failed to get depth");
    println!("Updated depth after removal:");
    println!("Bids:");
    for bid in &depth.bids {
        println!("  Price: {}, Volume: {}, Orders: {}", 
            bid.price,
            bid.volume,
            bid.order_count);
    }
    
    // Shutdown the worker
    println!("Shutting down worker...");
    client.shutdown().await.expect("Failed to shut down worker");
    println!("Done!");
} 
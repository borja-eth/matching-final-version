use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ultimate_matching::orderbook::OrderBook;
use ultimate_matching::matching_engine::MatchingEngine;
use ultimate_matching::types::{Order, Side, OrderType, OrderStatus, CreatedFrom, TimeInForce};
use rust_decimal_macros::dec;
use uuid::Uuid;
use chrono::Utc;
use rust_decimal::Decimal;

fn create_test_order(side: Side, price: Decimal, quantity: Decimal, instrument_id: Uuid) -> Order {
    let now = Utc::now();
    Order {
        id: Uuid::new_v4(),
        ext_id: Some("bench-order".to_string()),
        account_id: Uuid::new_v4(),
        order_type: OrderType::Limit,
        instrument_id,
        side,
        limit_price: Some(price.round_dp(0).try_into().unwrap()),
        trigger_price: None,
        base_amount: quantity.round_dp(0).try_into().unwrap(),
        remaining_base: quantity.round_dp(0).try_into().unwrap(),
        filled_quote: 0,
        filled_base: 0,
        remaining_quote: (price * quantity).round_dp(0).try_into().unwrap(),
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

fn orderbook_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_operations");
    
    // Benchmark adding orders
    group.bench_function("add_order", |b| {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
        
        b.iter(|| {
            let _ = book.add_order(black_box(order.clone()));
        });
    });
    
    // Benchmark removing orders
    group.bench_function("remove_order", |b| {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
        let _ = book.add_order(order.clone());
        
        // Extract limit price once outside the benchmark loop
        let _limit_price = match order.limit_price {
            Some(price) => price,
            None => panic!("Test order must have a limit price"),
        };
        
        b.iter(|| {
            let _ = book.remove_order(black_box(order.id));
        });
    });
    
    // Benchmark getting best prices
    group.bench_function("get_best_prices", |b| {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add some orders
        for i in 0..100 {
            let buy_price = Decimal::from(100 - i);
            let sell_price = Decimal::from(100 + i);
            
            let _ = book.add_order(create_test_order(
                Side::Bid,
                buy_price,
                dec!(1.0),
                instrument_id
            ));
            let _ = book.add_order(create_test_order(
                Side::Ask,
                sell_price,
                dec!(1.0),
                instrument_id
            ));
        }
        
        b.iter(|| {
            black_box(book.best_bid());
            black_box(book.best_ask());
        });
    });
    
    // Benchmark getting best bid and ask orders
    group.bench_function("get_best_bid_and_ask", |b| {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        
        // Add some orders
        for i in 0..100 {
            let buy_price = Decimal::from(100 - i);
            let sell_price = Decimal::from(100 + i);
            
            let _ = book.add_order(create_test_order(
                Side::Bid,
                buy_price,
                dec!(1.0),
                instrument_id
            ));
            let _ = book.add_order(create_test_order(
                Side::Ask,
                sell_price,
                dec!(1.0),
                instrument_id
            ));
        }
        
        b.iter(|| {
            black_box(book.get_best_bid());
            black_box(book.get_best_ask());
        });
    });
    
    group.finish();
}

fn matching_engine_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_engine_operations");
    
    // Benchmark processing a matching order
    group.bench_function("process_matching_order", |b| {
        let instrument_id = Uuid::new_v4();
        let mut engine = MatchingEngine::new(instrument_id);
        
        // Pre-populate with buy orders at different price levels
        for i in 0..5 {
            let buy_price = dec!(100.0) - Decimal::from(i);
            let buy_order = create_test_order(
                Side::Bid,
                buy_price,
                dec!(1.0),
                instrument_id
            );
            // We're not benchmarking this part, so ignore the result
            let _ = engine.process_order(buy_order, TimeInForce::GTC);
        }
        
        // Create a sell order that will match with the best bid
        let sell_order = create_test_order(
            Side::Ask,
            dec!(100.0),
            dec!(1.0),
            instrument_id
        );
        
        b.iter(|| {
            // Clone the order for each iteration to ensure a fresh state
            let order_clone = black_box(sell_order.clone());
            let _ = black_box(engine.process_order(order_clone, TimeInForce::GTC));
            
            // Repopulate the order book after each iteration
            let replenish_order = create_test_order(
                Side::Bid,
                dec!(100.0),
                dec!(1.0),
                instrument_id
            );
            let _ = engine.process_order(replenish_order, TimeInForce::GTC);
        });
    });
    
    group.finish();
}

// Function to print TPS estimates after benchmarks
fn print_tps_estimates() {
    println!("\n======= BENCHMARK RESULTS (TPS) =======");
    
    // Try to read and process each benchmark result
    let bench_dirs = [
        "target/criterion/matching_engine_operations/process_matching_order",
        "target/criterion/orderbook_operations/add_order",
        "target/criterion/orderbook_operations/remove_order",
        "target/criterion/orderbook_operations/get_best_prices",
        "target/criterion/orderbook_operations/get_best_bid_and_ask",
    ];
    
    for dir in bench_dirs {
        let file_path = format!("{}/new/estimates.json", dir);
        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(mean) = json.get("mean").and_then(|m| m.get("point_estimate")).and_then(|p| p.as_f64()) {
                        let ns_per_op = mean;
                        let tps = if ns_per_op > 0.0 { 1_000_000_000.0 / ns_per_op } else { 0.0 };
                        
                        // Extract operation name from directory
                        let parts: Vec<&str> = dir.split('/').collect();
                        let op_name = if parts.len() >= 3 { parts[parts.len() - 2] } else { "unknown" };
                        
                        println!("• {}: {:.2} TPS ({:.2} ns/op)", op_name, tps, ns_per_op);
                    }
                }
            }
            Err(_) => {
                // File not found or couldn't be read - might not have been run yet
            }
        }
    }
    
    println!("========================================\n");
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = orderbook_benchmark, matching_engine_benchmark
}

criterion_main! {
    benches
}

// Print TPS estimates whenever this module is loaded
#[ctor::ctor]
fn print_tps() {
    // Add a small delay to ensure criterion has time to write results
    std::thread::sleep(std::time::Duration::from_millis(100));
    print_tps_estimates();
} 
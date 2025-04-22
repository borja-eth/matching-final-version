use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use uuid::Uuid;
use chrono::Utc;
use std::time::{Duration, Instant};

use ultimate_matching::domain::models::types::{Order, Side, OrderType, OrderStatus, TimeInForce, CreatedFrom};
use ultimate_matching::domain::services::orderbook::orderbook::OrderBook;

/// Creates a test order with specified parameters
fn create_test_order(
    side: Side,
    price: i64,
    quantity: u64,
    instrument_id: Uuid,
) -> Order {
    let now = Utc::now();
    
    Order {
        id: Uuid::new_v4(),
        ext_id: Some("bench-order".to_string()),
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
        remaining_quote: (price * quantity as i64) as u64,
        expiration_date: now + chrono::Duration::days(1),
        status: OrderStatus::Submitted,
        created_at: now,
        updated_at: now,
        trigger_by: None,
        created_from: CreatedFrom::Api,
        sequence_id: 0,
        time_in_force: TimeInForce::GTC,
    }
}

/// Benchmarks orderbook throughput at different loads
fn bench_orderbook_throughput(c: &mut Criterion) {
    let instrument_id = Uuid::new_v4();
    let mut group = c.benchmark_group("orderbook_throughput");
    
    // Test different batch sizes
    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::new("add_orders", size), size, |b, &size| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::from_secs(0);
                
                for _ in 0..iters {
                    let mut orderbook = OrderBook::new(instrument_id);
                    let start = Instant::now();
                    
                    // Add orders with varying prices
                    for i in 0..size {
                        let side = if i % 2 == 0 { Side::Bid } else { Side::Ask };
                        let price_offset = (i % 10) as i64;
                        let price = if side == Side::Bid {
                            100_000 - price_offset * 100
                        } else {
                            100_000 + price_offset * 100
                        };
                        
                        let order = create_test_order(side, price, 1_000, instrument_id);
                        let _ = orderbook.add_order(order);
                    }
                    
                    total_duration += start.elapsed();
                }
                
                total_duration
            });
        });
        
        group.bench_with_input(BenchmarkId::new("mixed_operations", size), size, |b, &size| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::from_secs(0);
                
                for _ in 0..iters {
                    let mut orderbook = OrderBook::new(instrument_id);
                    let mut order_ids = Vec::with_capacity(size);
                    
                    // Pre-populate the orderbook
                    for i in 0..size {
                        let side = if i % 2 == 0 { Side::Bid } else { Side::Ask };
                        let price_offset = (i % 10) as i64;
                        let price = if side == Side::Bid {
                            100_000 - price_offset * 100
                        } else {
                            100_000 + price_offset * 100
                        };
                        
                        let order = create_test_order(side, price, 1_000, instrument_id);
                        order_ids.push(order.id);
                        let _ = orderbook.add_order(order);
                    }
                    
                    let start = Instant::now();
                    
                    // Mixed operations: 40% add, 30% remove, 30% lookup
                    for i in 0..size {
                        match i % 10 {
                            0..=3 => {
                                // Add new order
                                let side = if i % 2 == 0 { Side::Bid } else { Side::Ask };
                                let price = if side == Side::Bid { 99_900 } else { 100_100 };
                                let order = create_test_order(side, price, 1_000, instrument_id);
                                let _ = orderbook.add_order(order);
                            },
                            4..=6 => {
                                // Remove existing order
                                if !order_ids.is_empty() {
                                    let idx = i % order_ids.len();
                                    let _ = orderbook.remove_order(order_ids[idx]);
                                }
                            },
                            _ => {
                                // Lookup operations
                                let _ = orderbook.best_bid();
                                let _ = orderbook.best_ask();
                                
                                // Get orders at a specific price
                                let _ = orderbook.get_orders_at_price(Side::Bid, 99_900);
                            }
                        }
                    }
                    
                    total_duration += start.elapsed();
                }
                
                total_duration
            });
        });
        
        // Matching simulation
        group.bench_with_input(BenchmarkId::new("matching_simulation", size), size, |b, &size| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::from_secs(0);
                
                for _ in 0..iters {
                    let mut orderbook = OrderBook::new(instrument_id);
                    
                    // Create limit orders that will match
                    let mut orders = Vec::with_capacity(size);
                    
                    // Create buy/sell orders that will match
                    for i in 0..size {
                        let is_even = i % 2 == 0;
                        let side = if is_even { Side::Bid } else { Side::Ask };
                        let price = 100_000; // Same price for matching
                        
                        let order = create_test_order(side, price, 1_000, instrument_id);
                        orders.push(order);
                    }
                    
                    let start = Instant::now();
                    
                    // Process orders - this simulates matching
                    for order in orders {
                        let _ = orderbook.add_order(order);
                    }
                    
                    total_duration += start.elapsed();
                }
                
                total_duration
            });
        });
    }
    
    group.finish();
}

/// Benchmarks handling a large number of price levels
fn bench_price_levels(c: &mut Criterion) {
    let instrument_id = Uuid::new_v4();
    let mut group = c.benchmark_group("orderbook_price_levels");
    
    for levels in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*levels as u64));
        
        group.bench_with_input(BenchmarkId::new("many_price_levels", levels), levels, |b, &levels| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::from_secs(0);
                
                for _ in 0..iters {
                    let mut orderbook = OrderBook::new(instrument_id);
                    
                    // Add orders at many different price levels
                    for i in 0..levels {
                        let bid_price = 100_000 - (i as i64) * 10;
                        let ask_price = 100_000 + (i as i64) * 10;
                        
                        // Add a bid and ask at each level
                        let bid_order = create_test_order(Side::Bid, bid_price, 1_000, instrument_id);
                        let ask_order = create_test_order(Side::Ask, ask_price, 1_000, instrument_id);
                        
                        let _ = orderbook.add_order(bid_order);
                        let _ = orderbook.add_order(ask_order);
                    }
                    
                    let start = Instant::now();
                    
                    // Benchmark spread calculation and finding best levels
                    for _ in 0..100 {
                        let _ = orderbook.spread();
                        let _ = orderbook.best_bid();
                        let _ = orderbook.best_ask();
                        let _ = orderbook.get_best_bid();
                        let _ = orderbook.get_best_ask();
                    }
                    
                    total_duration += start.elapsed();
                }
                
                total_duration
            });
        });
    }
    
    group.finish();
}

/// Benchmarks contention on a single price level
fn bench_price_level_contention(c: &mut Criterion) {
    let instrument_id = Uuid::new_v4();
    let mut group = c.benchmark_group("price_level_contention");
    
    for orders_per_level in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*orders_per_level as u64));
        
        group.bench_with_input(BenchmarkId::new("single_price_level", orders_per_level), orders_per_level, |b, &orders_per_level| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::from_secs(0);
                
                for _ in 0..iters {
                    let mut orderbook = OrderBook::new(instrument_id);
                    let price = 100_000;
                    
                    // Add many orders at the same price level
                    for _ in 0..orders_per_level {
                        let order = create_test_order(Side::Bid, price, 1_000, instrument_id);
                        let _ = orderbook.add_order(order);
                    }
                    
                    let start = Instant::now();
                    
                    // Benchmark operations on a crowded price level
                    let level = orderbook.get_price_level(Side::Bid, price).unwrap();
                    let _ = level.total_volume;
                    let _ = level.order_count();
                    
                    total_duration += start.elapsed();
                }
                
                total_duration
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_orderbook_throughput,
    bench_price_levels,
    bench_price_level_contention
);
criterion_main!(benches); 
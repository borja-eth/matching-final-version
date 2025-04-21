use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;
use chrono::Utc;
use rand::{thread_rng, Rng};
use std::time::Duration;

use ultimate_matching::types::{Order, Side, OrderType, OrderStatus, TimeInForce, CreatedFrom};
use ultimate_matching::matching_engine::MatchingEngine;

fn create_test_order(
    side: Side,
    price: Decimal,
    quantity: Decimal,
    order_type: OrderType,
    tif: TimeInForce,
    instrument_id: Uuid,
) -> Order {
    let now = Utc::now();
    Order {
        id: Uuid::new_v4(),
        ext_id: Some("test-order".to_string()),
        account_id: Uuid::new_v4(),
        order_type,
        instrument_id,
        side,
        limit_price: Some(price),
        trigger_price: None,
        base_amount: quantity,
        remaining_base: quantity,
        filled_base: dec!(0),
        filled_quote: dec!(0),
        remaining_quote: price * quantity,
        expiration_date: now + chrono::Duration::days(1),
        status: OrderStatus::New,
        created_at: now,
        updated_at: now,
        trigger_by: None,
        created_from: CreatedFrom::Api,
        sequence_id: 0,
    }
}

fn create_random_buy_order(price_levels: u32, instrument_id: Uuid) -> Order {
    let mut rng = thread_rng();
    let price = Decimal::from(10000 + rng.gen_range(0..price_levels));
    let quantity = Decimal::from(1 + rng.gen_range(1..100));
    
    create_test_order(
        Side::Bid,
        price,
        quantity,
        OrderType::Limit,
        TimeInForce::GTC,
        instrument_id,
    )
}

fn create_random_sell_order(price_levels: u32, instrument_id: Uuid) -> Order {
    let mut rng = thread_rng();
    let price = Decimal::from(10000 + rng.gen_range(0..price_levels));
    let quantity = Decimal::from(1 + rng.gen_range(1..100));
    
    create_test_order(
        Side::Ask,
        price,
        quantity,
        OrderType::Limit,
        TimeInForce::GTC,
        instrument_id,
    )
}

fn bench_orderbook_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_add_order");
    group.measurement_time(Duration::from_secs(10));
    
    for size in [1000, 10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let instrument_id = Uuid::new_v4();
            let mut engine = MatchingEngine::new(instrument_id);
            let orders: Vec<Order> = (0..size)
                .map(|i| {
                    if i % 2 == 0 {
                        create_random_buy_order(100, instrument_id)
                    } else {
                        create_random_sell_order(100, instrument_id)
                    }
                })
                .collect();
                
            b.iter(|| {
                for order in &orders {
                    let _ = black_box(engine.process_order(order.clone(), TimeInForce::GTC));
                }
            });
        });
    }
    
    group.finish();
}

fn bench_orderbook_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_matching");
    group.measurement_time(Duration::from_secs(10));
    
    for num_matches in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*num_matches as u64));
        
        group.bench_with_input(BenchmarkId::from_parameter(num_matches), num_matches, |b, &num_matches| {
            let instrument_id = Uuid::new_v4();
            
            b.iter(|| {
                let mut engine = MatchingEngine::new(instrument_id);
                
                // First add non-crossing orders to build the book
                for i in 0..num_matches {
                    // Add buy orders at decreasing prices
                    let buy_order = create_test_order(
                        Side::Bid,
                        Decimal::from(9900 - i),
                        dec!(10),
                        OrderType::Limit,
                        TimeInForce::GTC,
                        instrument_id,
                    );
                    
                    // Add sell orders at increasing prices
                    let sell_order = create_test_order(
                        Side::Ask,
                        Decimal::from(10100 + i),
                        dec!(10),
                        OrderType::Limit,
                        TimeInForce::GTC,
                        instrument_id,
                    );
                    
                    let _ = black_box(engine.process_order(buy_order, TimeInForce::GTC));
                    let _ = black_box(engine.process_order(sell_order, TimeInForce::GTC));
                }
                
                // Now add crossing orders that will match
                for _ in 0..num_matches {
                    // Aggressive buy that crosses with existing sells
                    let matching_buy = create_test_order(
                        Side::Bid,
                        dec!(10200),  // Price higher than any sell
                        dec!(10),
                        OrderType::Limit,
                        TimeInForce::GTC,
                        instrument_id,
                    );
                    
                    let _ = black_box(engine.process_order(matching_buy, TimeInForce::GTC));
                }
            });
        });
    }
    
    group.finish();
}

fn bench_orderbook_cancel(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_cancel");
    group.measurement_time(Duration::from_secs(10));
    
    for size in [1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let instrument_id = Uuid::new_v4();
            let mut engine = MatchingEngine::new(instrument_id);
            
            // Generate and add orders
            let orders: Vec<Order> = (0..size)
                .map(|i| {
                    if i % 2 == 0 {
                        create_random_buy_order(100, instrument_id)
                    } else {
                        create_random_sell_order(100, instrument_id)
                    }
                })
                .collect();
            
            // First add all orders to the book
            for order in &orders {
                let _ = engine.process_order(order.clone(), TimeInForce::GTC);
            }
            
            // Now benchmark cancellation
            b.iter(|| {
                for order in &orders {
                    let _ = black_box(engine.cancel_order(order.id));
                }
            });
        });
    }
    
    group.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    group.measurement_time(Duration::from_secs(15));
    
    for ops in [1000, 10000].iter() {
        group.throughput(Throughput::Elements(*ops as u64));
        
        group.bench_with_input(BenchmarkId::from_parameter(ops), ops, |b, &ops| {
            let instrument_id = Uuid::new_v4();
            
            b.iter(|| {
                let mut engine = MatchingEngine::new(instrument_id);
                let mut order_ids = Vec::with_capacity(ops as usize);
                
                for i in 0..ops {
                    let action = i % 10; // Distribute actions with specific frequencies
                    
                    match action {
                        0..=5 => {
                            // 60% chance: add a new order
                            let mut rng = thread_rng();
                            let side = if rng.gen_bool(0.5) {
                                Side::Bid
                            } else {
                                Side::Ask
                            };
                            
                            let price_offset = rng.gen_range(0..100);
                            let base_price = match side {
                                Side::Bid => Decimal::from(10000 - price_offset),
                                Side::Ask => Decimal::from(10000 + price_offset),
                            };
                            
                            let order = create_test_order(
                                side,
                                base_price,
                                Decimal::from(1 + rng.gen_range(1..100)),
                                OrderType::Limit,
                                TimeInForce::GTC,
                                instrument_id,
                            );
                            
                            let _ = black_box(engine.process_order(order.clone(), TimeInForce::GTC));
                            order_ids.push(order.id);
                        },
                        6..=7 => {
                            // 20% chance: add a matching order
                            let mut rng = thread_rng();
                            let side = if rng.gen_bool(0.5) {
                                Side::Bid
                            } else {
                                Side::Ask
                            };
                            
                            let price = match side {
                                Side::Bid => dec!(10100),  // Above typical sell price
                                Side::Ask => dec!(9900),   // Below typical buy price
                            };
                            
                            let order = create_test_order(
                                side,
                                price,
                                Decimal::from(1 + rng.gen_range(1..50)),
                                OrderType::Limit,
                                TimeInForce::GTC,
                                instrument_id,
                            );
                            
                            let _ = black_box(engine.process_order(order.clone(), TimeInForce::GTC));
                        },
                        8..=9 => {
                            // 20% chance: cancel an order
                            if !order_ids.is_empty() {
                                let mut rng = thread_rng();
                                let idx = rng.gen_range(0..order_ids.len());
                                let order_id = order_ids[idx];
                                let _ = black_box(engine.cancel_order(order_id));
                            }
                        },
                        _ => {}
                    }
                }
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_orderbook_add,
    bench_orderbook_matching,
    bench_orderbook_cancel,
    bench_mixed_workload
);
criterion_main!(benches); 
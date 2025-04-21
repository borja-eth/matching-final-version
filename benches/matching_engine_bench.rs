use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal_macros::dec;
use uuid::Uuid;
use chrono::Utc;

use ultimate_matching::types::{Order, Side, OrderType, OrderStatus, TimeInForce, CreatedFrom};
use ultimate_matching::matching_engine::MatchingEngine;

fn create_test_order(
    side: Side,
    order_type: OrderType,
    price: Option<rust_decimal::Decimal>,
    quantity: rust_decimal::Decimal,
    instrument_id: Uuid,
) -> Order {
    let now = Utc::now();
    let remaining_quote = match price {
        Some(p) => p * quantity,
        None => dec!(0),
    };

    Order {
        id: Uuid::new_v4(),
        ext_id: Some("test-order".to_string()),
        account_id: Uuid::new_v4(),
        order_type,
        instrument_id,
        side,
        limit_price: price,
        trigger_price: None,
        base_amount: quantity,
        remaining_base: quantity,
        filled_quote: dec!(0.0),
        filled_base: dec!(0.0),
        remaining_quote,
        expiration_date: now + chrono::Duration::days(365),
        status: OrderStatus::New,
        created_at: now,
        updated_at: now,
        trigger_by: None,
        created_from: CreatedFrom::Api,
        sequence_id: 0,
    }
}

fn setup_engine() -> (MatchingEngine, Uuid) {
    let instrument_id = Uuid::new_v4();
    let engine = MatchingEngine::new(instrument_id);
    (engine, instrument_id)
}

fn bench_mixed_workload(c: &mut Criterion) {
    let (mut engine, instrument_id) = setup_engine();
    let mut group = c.benchmark_group("mixed_workload");

    group.bench_function("realistic_mixed_operations", |b| {
        b.iter(|| {
            // 60% new orders
            for _ in 0..6 {
                let order = create_test_order(
                    Side::Bid,
                    OrderType::Limit,
                    Some(dec!(100.0)),
                    dec!(1.0),
                    instrument_id,
                );
                black_box(engine.process_order(order, TimeInForce::GTC).unwrap());
            }

            // 20% cancellations
            for _ in 0..2 {
                let order = create_test_order(
                    Side::Ask,
                    OrderType::Limit,
                    Some(dec!(101.0)),
                    dec!(1.0),
                    instrument_id,
                );
                let result = engine.process_order(order, TimeInForce::GTC).unwrap();
                if let Some(order) = result.processed_order {
                    black_box(engine.cancel_order(order.id).unwrap());
                }
            }

            // 20% matches
            for _ in 0..2 {
                let buy_order = create_test_order(
                    Side::Bid,
                    OrderType::Limit,
                    Some(dec!(100.0)),
                    dec!(1.0),
                    instrument_id,
                );
                let sell_order = create_test_order(
                    Side::Ask,
                    OrderType::Limit,
                    Some(dec!(100.0)),
                    dec!(1.0),
                    instrument_id,
                );
                black_box(engine.process_order(buy_order, TimeInForce::GTC).unwrap());
                black_box(engine.process_order(sell_order, TimeInForce::GTC).unwrap());
            }
        })
    });

    group.finish();
}

fn bench_high_frequency_matching(c: &mut Criterion) {
    let (mut engine, instrument_id) = setup_engine();
    let mut group = c.benchmark_group("high_frequency_matching");

    // Pre-fill order book
    for i in 0..10 {
        let price = dec!(100.0) + rust_decimal::Decimal::from(i);
        for _ in 0..100 {
            let order = create_test_order(
                Side::Ask,
                OrderType::Limit,
                Some(price),
                dec!(1.0),
                instrument_id,
            );
            engine.process_order(order, TimeInForce::GTC).unwrap();
        }
    }

    group.bench_function("rapid_matching", |b| {
        b.iter(|| {
            let order = create_test_order(
                Side::Bid,
                OrderType::Limit,
                Some(dec!(110.0)),
                dec!(5.0),
                instrument_id,
            );
            black_box(engine.process_order(order, TimeInForce::GTC).unwrap());
        })
    });

    group.finish();
}

fn bench_market_stress(c: &mut Criterion) {
    let (mut engine, instrument_id) = setup_engine();
    let mut group = c.benchmark_group("market_stress");

    group.bench_function("high_volatility", |b| {
        b.iter(|| {
            // Rapid price movement simulation
            for i in 0..5 {
                let price = dec!(100.0) + rust_decimal::Decimal::from(i);
                let buy_order = create_test_order(
                    Side::Bid,
                    OrderType::Limit,
                    Some(price),
                    dec!(1.0),
                    instrument_id,
                );
                let sell_order = create_test_order(
                    Side::Ask,
                    OrderType::Limit,
                    Some(price),
                    dec!(1.0),
                    instrument_id,
                );
                black_box(engine.process_order(buy_order, TimeInForce::GTC).unwrap());
                black_box(engine.process_order(sell_order, TimeInForce::GTC).unwrap());
            }
        })
    });

    group.finish();
}

fn bench_order_book_depth(c: &mut Criterion) {
    let (mut engine, instrument_id) = setup_engine();
    let mut group = c.benchmark_group("order_book_depth");

    // Pre-fill order book with deep levels
    for i in 0..50 {
        let price = dec!(100.0) + rust_decimal::Decimal::from(i);
        for _ in 0..20 {
            let order = create_test_order(
                Side::Ask,
                OrderType::Limit,
                Some(price),
                dec!(1.0),
                instrument_id,
            );
            engine.process_order(order, TimeInForce::GTC).unwrap();
        }
    }

    group.bench_function("deep_book_operations", |b| {
        b.iter(|| {
            let order = create_test_order(
                Side::Bid,
                OrderType::Limit,
                Some(dec!(125.0)),
                dec!(1.0),
                instrument_id,
            );
            black_box(engine.process_order(order, TimeInForce::GTC).unwrap());
        })
    });

    group.finish();
}

fn bench_ioc_orders(c: &mut Criterion) {
    let (mut engine, instrument_id) = setup_engine();
    let mut group = c.benchmark_group("ioc_orders");

    // Pre-fill order book
    for i in 0..5 {
        let price = dec!(100.0) + rust_decimal::Decimal::from(i);
        let order = create_test_order(
            Side::Ask,
            OrderType::Limit,
            Some(price),
            dec!(1.0),
            instrument_id,
        );
        engine.process_order(order, TimeInForce::GTC).unwrap();
    }

    group.bench_function("ioc_processing", |b| {
        b.iter(|| {
            let order = create_test_order(
                Side::Bid,
                OrderType::Limit,
                Some(dec!(102.0)),
                dec!(1.0),
                instrument_id,
            );
            black_box(engine.process_order(order, TimeInForce::IOC).unwrap());
        })
    });

    group.finish();
}

fn bench_market_orders(c: &mut Criterion) {
    let (mut engine, instrument_id) = setup_engine();
    let mut group = c.benchmark_group("market_orders");

    group.bench_function("market_order_processing", |b| {
        b.iter_with_setup(
            // Setup: Fill order book before each iteration
            || {
                // Clear previous orders by creating a new engine
                let (mut engine, instrument_id) = setup_engine();
                
                // Add sufficient liquidity at multiple price levels
                for i in 0..5 {
                    let price = dec!(100.0) + rust_decimal::Decimal::from(i);
                    let order = create_test_order(
                        Side::Ask,
                        OrderType::Limit,
                        Some(price),
                        dec!(10.0), // Plenty of liquidity
                        instrument_id,
                    );
                    engine.process_order(order, TimeInForce::GTC).unwrap();
                }
                (engine, instrument_id)
            },
            // Benchmark: Process market order
            |(mut engine, instrument_id)| {
                let order = create_test_order(
                    Side::Bid,
                    OrderType::Market,
                    None,
                    dec!(1.0),
                    instrument_id,
                );
                black_box(engine.process_order(order, TimeInForce::IOC).unwrap())
            }
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mixed_workload,
    bench_high_frequency_matching,
    bench_market_stress,
    bench_order_book_depth,
    bench_ioc_orders,
    bench_market_orders,
);
criterion_main!(benches); 
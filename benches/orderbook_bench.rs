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
        limit_price: Some(price),
        trigger_price: None,
        base_amount: quantity,
        remaining_base: quantity,
        filled_quote: dec!(0.0),
        filled_base: dec!(0.0),
        remaining_quote: price * quantity,
        expiration_date: now + chrono::Duration::days(365),
        status: OrderStatus::New,
        created_at: now,
        updated_at: now,
        trigger_by: None,
        created_from: CreatedFrom::Api,
        sequence_id: 1,
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
            book.add_order(black_box(order.clone()));
        });
    });
    
    // Benchmark removing orders
    group.bench_function("remove_order", |b| {
        let instrument_id = Uuid::new_v4();
        let mut book = OrderBook::new(instrument_id);
        let order = create_test_order(Side::Bid, dec!(100.0), dec!(1.0), instrument_id);
        book.add_order(order.clone());
        
        // Extract limit price once outside the benchmark loop
        let limit_price = match order.limit_price {
            Some(price) => price,
            None => panic!("Test order must have a limit price"),
        };
        
        b.iter(|| {
            book.remove_order(black_box(order.id), black_box(order.side), black_box(limit_price));
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
            
            book.add_order(create_test_order(
                Side::Bid,
                buy_price,
                dec!(1.0),
                instrument_id
            ));
            book.add_order(create_test_order(
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
            
            book.add_order(create_test_order(
                Side::Bid,
                buy_price,
                dec!(1.0),
                instrument_id
            ));
            book.add_order(create_test_order(
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

criterion_group!(benches, orderbook_benchmark, matching_engine_benchmark);
criterion_main!(benches); 
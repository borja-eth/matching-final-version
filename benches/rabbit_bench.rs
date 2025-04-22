use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::sync::oneshot;
use tokio::runtime::Runtime;
use uuid::Uuid;

use ultimate_matching::{
    Config, 
    OrderbookManagerServiceImpl,
    domain::models::types::{Side, OrderType, TimeInForce},
    inbounds::dtos::PlaceOrderRequest
};
use rabbitmq::{PublisherMode, RabbitMQBuilder, SubscriberMode, Message, PublisherContext};
use tokio::time;

// Initialize tracing
fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
}

// Benchmark for RabbitMQ publishing performance
fn bench_rabbit_publishing(c: &mut Criterion) {
    println!("Starting RabbitMQ publishing benchmark");
    init_tracing();
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("rabbit_publishing");
    group.sample_size(10); // Fewer samples due to external dependency
    group.measurement_time(Duration::from_secs(10));
    
    group.bench_function("publish_100_messages", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = black_box(publish_messages(100).await);
            });
        });
    });
    
    group.bench_function("publish_1000_messages", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = black_box(publish_messages(1000).await);
            });
        });
    });
    
    group.finish();
}

// Benchmark for RabbitMQ end-to-end (publish->subscribe->process) performance
fn bench_rabbit_end_to_end(c: &mut Criterion) {
    println!("Starting RabbitMQ end-to-end benchmark");
    init_tracing();
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("rabbit_end_to_end");
    group.sample_size(10); // Fewer samples due to external dependency
    group.measurement_time(Duration::from_secs(15));
    
    group.bench_function("process_100_orders", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = black_box(end_to_end_test(100).await);
            });
        });
    });
    
    group.finish();
}

// Helper function to publish messages
async fn publish_messages(count: usize) -> Result<(), String> {
    println!("Publishing {} messages", count);
    // Load configuration
    let config = Config::from_env();
    let app_id = config.app_id.clone();
    let rabbit_url = config.rabbit_url.clone();
    let instrument_id = config.instruments[0]; // Use the first instrument
    
    println!("Using RabbitMQ URL: {}", rabbit_url);
    println!("App ID: {}", app_id);
    
    // Build RabbitMQ client for publishing
    let builder = RabbitMQBuilder::new(&rabbit_url, &app_id)
        .publisher("Results", PublisherMode::Topic);
    
    let client = match builder.build().await {
        Ok(client) => {
            println!("Successfully built RabbitMQ client");
            client
        },
        Err(e) => return Err(format!("Failed to build RabbitMQ client: {}", e)),
    };
    
    let mut publishers = client.get_publishers();
    
    // Set up results publisher
    let publisher = match publishers.take_ownership(("Results", PublisherMode::Topic)) {
        Ok(publisher) => {
            println!("Successfully created publisher");
            publisher
        },
        Err(err) => return Err(format!("Failed to create publisher: {}", err)),
    };
    
    // Create test messages
    let place_order_channel = format!("matching.orders.{}.place", instrument_id);
    
    // Publish messages
    for i in 0..count {
        let order_request = create_test_order_request(i as u64, instrument_id);
        let content = serde_json::to_string(&order_request).unwrap();
        
        // Create message and publish context
        let message = Message::new(content.as_bytes().to_vec(), Some(place_order_channel.clone()));
        let context = PublisherContext::new(&format!("bench-{}", i), Some(format!("msg-{}", i)));
        
        if let Err(e) = publisher.publish(message, context) {
            return Err(format!("Failed to publish message: {}", e));
        }
        
        // Small delay to avoid overwhelming the broker
        if i % 100 == 0 {
            println!("Published {} messages", i);
            time::sleep(Duration::from_millis(5)).await;
        }
    }
    
    // Add a small delay to ensure all messages are delivered
    time::sleep(Duration::from_millis(100)).await;
    println!("Successfully published {} messages", count);
    
    Ok(())
}

// Helper function for end-to-end test (publish -> subscribe -> process)
async fn end_to_end_test(count: usize) -> Result<(), String> {
    println!("Starting end-to-end test with {} messages", count);
    // Load configuration
    let config = Config::from_env();
    let app_id = config.app_id.clone();
    let rabbit_url = config.rabbit_url.clone();
    let instrument_id = config.instruments[0]; // Use the first instrument
    
    // Set up order tracking
    let processed_count = Arc::new(AtomicU64::new(0));
    let processed_count_clone = processed_count.clone();
    
    // Set up shutdown channel
    let (tx, rx) = oneshot::channel();
    
    // Set up orderbook manager
    let orderbook_manager = Arc::new(OrderbookManagerServiceImpl::new(config.instruments.clone()));
    
    // Start subscriber in a separate task
    let join_handle = tokio::spawn(async move {
        let place_order_channel = format!("matching.orders.{}.place", instrument_id);
        println!("Subscribing to channel: {}", place_order_channel);
        
        // Build subscriber
        let builder = RabbitMQBuilder::new(&rabbit_url, &app_id)
            .subscriber(
                "Orders",
                SubscriberMode::Topics {
                    topics: vec![place_order_channel.clone()],
                },
            );
        
        let server = match builder.build().await {
            Ok(server) => {
                println!("Successfully built RabbitMQ server");
                server
            },
            Err(e) => {
                return Err(format!("Failed to build RabbitMQ server: {}", e));
            }
        };
        
        let mut subscribers = server.get_subscribers();
        
        // Set up subscriber
        let mut subscriber = match subscribers.take_ownership((
            "Orders",
            SubscriberMode::Topics {
                topics: vec![place_order_channel],
            },
        )) {
            Ok(subscriber) => {
                println!("Successfully created subscriber");
                subscriber
            },
            Err(e) => {
                return Err(format!("Failed to set up subscriber: {}", e));
            }
        };
        
        // Process messages
        let mut rx = rx;
        println!("Starting to process messages");
        loop {
            tokio::select! {
                // Check for shutdown signal
                shutdown = &mut rx => {
                    if shutdown.is_ok() {
                        println!("Received shutdown signal");
                        break;
                    }
                }
                // Process messages
                msg = subscriber.receive() => {
                    if let Some(msg) = msg {
                        if let Some(content) = msg.content.clone() {
                            // Process order
                            match ultimate_matching::inbounds::handlers::place_handler::handle_place_request(
                                content, 
                                orderbook_manager.clone()
                            ) {
                                Ok(_) => {
                                    let count = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
                                    if count % 10 == 0 {
                                        println!("Processed {} messages", count);
                                    }
                                },
                                Err(e) => println!("Error processing message: {}", e),
                            }
                        }
                        
                        // Acknowledge message
                        if let Err(e) = subscriber.ack(&msg).await {
                            println!("Error acknowledging message: {}", e);
                        }
                    }
                }
            }
        }
        
        Ok(())
    });
    
    // Wait for subscriber to start
    println!("Waiting for subscriber to start");
    time::sleep(Duration::from_millis(500)).await;
    
    // Publish test messages
    println!("Publishing test messages");
    publish_messages(count).await?;
    
    // Wait for messages to be processed
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(10);
    
    println!("Waiting for messages to be processed");
    loop {
        let processed = processed_count_clone.load(Ordering::SeqCst);
        println!("Processed {} out of {} messages", processed, count);
        if processed >= count as u64 || start_time.elapsed() > timeout {
            break;
        }
        time::sleep(Duration::from_millis(100)).await;
    }
    
    // Send shutdown signal
    println!("Sending shutdown signal");
    let _ = tx.send(());
    
    // Wait for subscriber to shut down
    println!("Waiting for subscriber to shut down");
    let _ = join_handle.await;
    
    println!("End-to-end test completed");
    Ok(())
}

// Helper function to create a test order request
fn create_test_order_request(id: u64, instrument_id: Uuid) -> PlaceOrderRequest {
    PlaceOrderRequest {
        ext_id: Some(format!("bench-{}", id)),
        account_id: Uuid::new_v4(),
        instrument: instrument_id,
        side: Side::Bid,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::GTC,
        limit_price: Some(10000),
        trigger_price: None,
        base_amount: 100,
        version: 1,
        request_type: "bench".to_string(),
        new_order_id: Uuid::new_v4(),
    }
}

criterion_group!(
    name = rabbit_benches;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(10));
    targets = bench_rabbit_publishing, bench_rabbit_end_to_end
);
criterion_main!(rabbit_benches); 
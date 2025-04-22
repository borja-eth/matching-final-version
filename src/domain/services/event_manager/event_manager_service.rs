//! Implementation of the event manager service that processes orderbook results
//! and publishes them as events to external systems.

use std::{sync::Arc, thread};

use async_trait::async_trait;
use chrono::Utc;
use rabbitmq::{Publisher, PublisherMode, RabbitMQBuilder};
use tokio::{runtime::Runtime, sync::mpsc::Receiver};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    Config,
    domain::{
        models::orderbook::{AddOrderResult, CancelledOrderResult, OrderbookResult, OrderbookSnapshot},
        services::orderbook::OrderbookError,
    },
    outbounds::{
        events::{
            market::{
                Level1Update, Level2Delta, MarketEventType, MessageType, OrderbookSnapshot as MarketOrderbookSnapshot, 
                TradingSessionStatus, TradingStatus,
            },
            order::{MatchEvent, OrderAcknowledgement, OrderCancel, OrderReject, ResultEvent},
        },
        handle_event::handle_event,
    },
};

use super::{EventManagerError, EventManagerService};

/// Implementation of the event manager service
/// 
/// This service is responsible for:
/// 1. Receiving orderbook results from orderbooks
/// 2. Converting them to event types
/// 3. Publishing them to external systems via RabbitMQ
pub struct EventManagerServiceImpl {
    /// RabbitMQ publisher for sending events
    rabbit_publisher: Arc<Publisher>,
    
    /// List of instrument IDs this event manager is handling
    instruments: Vec<Uuid>,
}

impl EventManagerServiceImpl {
    /// Creates a new event manager service
    ///
    /// # Arguments
    /// * `config` - Application configuration containing RabbitMQ settings
    ///
    /// # Returns
    /// A new EventManagerServiceImpl instance or panics if the RabbitMQ connection fails
    pub async fn new(config: &Config) -> Self {
        // Create RabbitMQ builder with configuration
        let builder = RabbitMQBuilder::new(&config.rabbit_url, &config.app_id)
            .publisher("events", PublisherMode::Broadcast);

        // Build the client (publisher only)
        let client = match builder.build().await {
            Ok(client) => client,
            Err(err) => {
                panic!(
                    "Error getting rabbit publisher in event manager service: {}",
                    err
                )
            }
        };

        // Get the publishers, which consumes the client
        let mut client_queues = client.get_publishers();

        // Ensure we can get the publisher
        let rabbit_publisher = client_queues
            .take_ownership(("events", PublisherMode::Broadcast))
            .expect("Failed to get publisher in event manager service");

        Self {
            rabbit_publisher: Arc::new(rabbit_publisher),
            instruments: config.instruments.clone(),
        }
    }
}

#[async_trait]
impl EventManagerService for EventManagerServiceImpl {
    fn run(&self, result_receiver: Receiver<(Uuid, OrderbookResult)>) -> thread::JoinHandle<()> {
        // Create a runtime for async processing
        let rt = Runtime::new().expect("Failed to create runtime for event manager");

        let mut result_receiver = result_receiver;
        let publisher = self.rabbit_publisher.clone();

        thread::spawn(move || {
            rt.block_on(async move {
                info!("Event manager starting processing results");
                
                // Process events until the receiver is closed
                while let Some(event) = result_receiver.recv().await {
                    let instrument_id = event.0;
                    let events = match event.1 {
                        OrderbookResult::Add(ref add_result) => {
                            process_add_result(add_result.clone(), instrument_id)
                        }
                        OrderbookResult::Cancelled(ref cancel_result) => {
                            process_cancel_result(cancel_result.clone(), instrument_id)
                        }
                        OrderbookResult::Error(ref error) => {
                            process_error(error)
                        }
                        OrderbookResult::Halted => {
                            build_trading_status_event(TradingStatus::Halted, instrument_id)
                        }
                        OrderbookResult::Resumed => {
                            build_trading_status_event(TradingStatus::Running, instrument_id)
                        }
                        OrderbookResult::Snapshot(ref snapshot) => {
                            build_snapshot_event(snapshot.clone(), instrument_id)
                        }
                    };

                    // Publish events
                    handle_event(events, publisher.clone());
                }
                
                info!("Event manager finished processing results");
            })
        })
    }

    async fn start(&self) -> Result<(), EventManagerError> {
        info!("Event manager service starting");
        Ok(())
    }
}

/// Processes an add order result and converts it to events
///
/// # Arguments
/// * `add_result` - Result of adding an order to the orderbook
/// * `instrument_id` - ID of the instrument
///
/// # Returns
/// Vector of result events to publish
fn process_add_result(add_result: AddOrderResult, instrument_id: Uuid) -> Vec<ResultEvent> {
    let mut events = Vec::new();

    // Extract and clone what we need first before moving parts of add_result
    let best_bid_and_ask = add_result.best_bid_and_ask.clone();
    let depth_changes_clone = if !add_result.depth_changes.is_empty() {
        // Convert to format needed for L2 delta
        let depth_changes: Vec<(i64, u64, crate::domain::models::types::Side)> = 
            add_result.depth_changes.values()
                .map(|level| (level.price, level.volume, match level.price {
                    p if p > 0 => crate::domain::models::types::Side::Ask,
                    _ => crate::domain::models::types::Side::Bid,
                }))
                .collect();
        Some(depth_changes)
    } else {
        None
    };

    // Process new order acknowledgment
    if let Some(new_order) = add_result.new_order {
        events.push(ResultEvent::OrderAck(OrderAcknowledgement {
            order_id: new_order.id,
            new_status: new_order.status.into(),
            seq_num: generate_sequence_number(),
            timestamp: Utc::now(),
        }));
    }

    // Process matches
    for match_data in add_result.matches {
        events.push(ResultEvent::Match(MatchEvent {
            taker_order_id: match_data.taker_order_id,
            maker_order_id: match_data.maker_order_id,
            taker_account_id: match_data.taker_account_id,
            maker_account_id: match_data.maker_account_id,
            maker_status: match_data.maker_status.into(),
            taker_status: match_data.taker_status.into(),
            match_base_amount: match_data.match_base_amount,
            match_quote_amount: match_data.match_quote_amount,
            timestamp: Utc::now(),
            seq_num: match_data.seq_num,
            match_price: match_data.limit_price,
        }));
    }

    // Process rejected orders
    for rejected_order in add_result.rejected_orders.iter() {
        events.push(ResultEvent::OrderReject(OrderReject {
            order_id: rejected_order.id,
            symbol: instrument_id.to_string(),
            reason: "Order rejected by orderbook".to_string(),
            timestamp: Utc::now(),
            seq_num: generate_sequence_number(),
            error_code: 1,
            new_status: rejected_order.status.into(),
        }));
    }

    // Process L2 update if we have depth changes
    if let Some(depth_changes) = depth_changes_clone {
        let l2_delta = create_l2_delta_update(depth_changes, instrument_id);
        events.push(ResultEvent::L2Delta(l2_delta));
    }
    
    // Process L1 update
    let l1_update = create_l1_update(best_bid_and_ask, instrument_id);
    events.push(ResultEvent::L1Update(l1_update));

    events
}

/// Processes a cancel order result and converts it to events
///
/// # Arguments
/// * `cancel_result` - Result of cancelling an order
/// * `instrument_id` - ID of the instrument
///
/// # Returns
/// Vector of result events to publish
fn process_cancel_result(cancel_result: CancelledOrderResult, instrument_id: Uuid) -> Vec<ResultEvent> {
    let mut events = Vec::new();
    
    // Create cancel event
    let order = cancel_result.order;
    events.push(ResultEvent::OrderCancel(OrderCancel {
        order_id: order.id,
        symbol: instrument_id.to_string(),
        reason: "Order cancelled".to_string(),
        timestamp: Utc::now(),
        seq_num: generate_sequence_number(),
        filled_base: order.filled_base,
        filled_quote: order.filled_quote,
        remaining_quantity: Some(order.remaining_base),
    }));

    // Process L2 update (depth change)
    let l2_delta = create_l2_delta_update(
        vec![(cancel_result.depth_changes.price, 
              cancel_result.depth_changes.volume, 
              if cancel_result.depth_changes.price > 0 {
                  crate::domain::models::types::Side::Ask
              } else {
                  crate::domain::models::types::Side::Bid
              })],
        instrument_id
    );
    events.push(ResultEvent::L2Delta(l2_delta));
    
    // Process L1 update (best bid/ask)
    let l1_update = create_l1_update(cancel_result.best_bid_and_ask, instrument_id);
    events.push(ResultEvent::L1Update(l1_update));

    events
}

/// Processes an error and logs it
///
/// # Arguments
/// * `error` - Orderbook error
///
/// # Returns
/// Empty vector of events
fn process_error(error: &OrderbookError) -> Vec<ResultEvent> {
    warn!("Event manager received error: {:?}", error);
    vec![]
}

/// Builds a trading session status event
///
/// # Arguments
/// * `status` - Trading status (Running or Halted)
/// * `instrument_id` - ID of the instrument
///
/// # Returns
/// Vector containing a single trading session status event
fn build_trading_status_event(status: TradingStatus, instrument_id: Uuid) -> Vec<ResultEvent> {
    // Define which message types are accepted based on status
    let mut accepted_messages = vec![MessageType::CancelOrder];
    if status == TradingStatus::Running {
        accepted_messages.push(MessageType::NewOrder);
    }

    // Create the trading status event
    let trading_status = TradingSessionStatus {
        version: 1,
        event_type: MarketEventType::TradingSessionStatus,
        timestamp: Utc::now(),
        status,
        accepted_messages,
        instrument_id,
    };

    vec![ResultEvent::TradingStatus(trading_status)]
}

/// Builds a snapshot event
///
/// # Arguments
/// * `snapshot` - Depth snapshot from the orderbook
/// * `instrument_id` - ID of the instrument
///
/// # Returns
/// Vector containing a single snapshot event
fn build_snapshot_event(snapshot: OrderbookSnapshot, instrument_id: Uuid) -> Vec<ResultEvent> {
    // Generate sequence number
    let seq_num = generate_sequence_number();

    // Convert our depth levels to market format
    let mut bids = Vec::new();
    let mut asks = Vec::new();
    
    for level in snapshot.depth_levels {
        // Determine side based on price (positive for asks, negative for bids)
        if level.price > 0 {
            asks.push((level.price, level.volume));
        } else {
            bids.push((level.price, level.volume));
        }
    }

    // Create the snapshot event
    let snapshot_event = MarketOrderbookSnapshot {
        version: 1,
        event_type: MarketEventType::Snapshot,
        bids,
        asks,
        seq_num,
        timestamp: Utc::now(),
        instrument_id,
    };

    vec![ResultEvent::Snapshot(snapshot_event)]
}

/// Creates a Level2Delta event from depth data
///
/// # Arguments
/// * `depth_changes` - List of price level changes (price, quantity, side)
/// * `instrument_id` - ID of the instrument
///
/// # Returns
/// Level2Delta event
fn create_l2_delta_update(depth_changes: Vec<(i64, u64, crate::domain::models::types::Side)>, instrument_id: Uuid) -> Level2Delta {
    // Generate sequence number
    let seq_num = generate_sequence_number();
    let timestamp = generate_timestamp();
    
    // Separate changes by side
    let mut bids = Vec::new();
    let mut asks = Vec::new();
    
    for (price, quantity, side) in depth_changes {
        match side {
            crate::domain::models::types::Side::Bid => bids.push((price, quantity)),
            crate::domain::models::types::Side::Ask => asks.push((price, quantity)),
        }
    }

    // Create L2Delta
    Level2Delta {
        version: 1,
        event_type: MarketEventType::Level2,
        bids,
        asks,
        seq_num,
        timestamp,
        instrument_id,
    }
}

/// Creates a Level1Update event from best bid/ask data
///
/// # Arguments
/// * `best_bid_and_ask` - Best bid and ask from the orderbook
/// * `instrument_id` - ID of the instrument
///
/// # Returns
/// Level1Update event
fn create_l1_update(best_bid_and_ask: crate::domain::models::orderbook::BestBidAndAsk, instrument_id: Uuid) -> Level1Update {
    // Generate sequence number
    let seq_num = generate_sequence_number();
    let timestamp = generate_timestamp();
    
    // Convert best bid/ask to expected format
    let bid = best_bid_and_ask.best_bid.map(|price| (price, 0u64)); // We don't have quantity in BestBidAndAsk
    let ask = best_bid_and_ask.best_ask.map(|price| (price, 0u64));

    // Create L1Update
    Level1Update {
        version: 1,
        event_type: MarketEventType::Level1,
        bid,
        ask,
        seq_num,
        timestamp,
        instrument_id,
    }
}

/// Generates a sequence number based on timestamp
///
/// # Returns
/// Sequence number as u64
fn generate_sequence_number() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Generates a timestamp for events
///
/// # Returns
/// Timestamp as u64 (nanoseconds since epoch)
fn generate_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

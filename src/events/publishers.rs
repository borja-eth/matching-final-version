use crate::matching_engine::{MatchingEngine, MatchResult};
use crate::orderbook::OrderBook;
use crate::types::{Order, Side, Trade};
use crate::events::types::{Event, OrderEvent, TradeEvent, OrderBookEvent};
use crate::events::bus::EventBus;
use rust_decimal::Decimal;
use std::sync::Arc;

/// Trait for components that publish events
pub trait EventPublisher {
    /// Publish events related to component's operations
    fn publish_events(&self);
}

/// Publisher for matching engine events
pub struct MatchingEnginePublisher {
    /// Reference to the event bus
    event_bus: EventBus,
    /// The matching engine being monitored
    engine: Arc<parking_lot::RwLock<MatchingEngine>>,
    /// Last observed state, used to detect changes
    last_state: parking_lot::Mutex<MatchingEngineState>,
}

/// Tracks the state of the matching engine for event generation
struct MatchingEngineState {
    /// Last known set of orders in the engine
    orders: Vec<Order>,
    /// Last known trades
    trades: Vec<Trade>,
    /// Last known best bid price
    best_bid: Option<Decimal>,
    /// Last known best ask price
    best_ask: Option<Decimal>,
}

impl MatchingEnginePublisher {
    /// Creates a new publisher for the matching engine
    pub fn new(event_bus: EventBus, engine: Arc<parking_lot::RwLock<MatchingEngine>>) -> Self {
        Self {
            event_bus,
            engine,
            last_state: parking_lot::Mutex::new(MatchingEngineState {
                orders: Vec::new(),
                trades: Vec::new(),
                best_bid: None,
                best_ask: None,
            }),
        }
    }

    /// Processes a match result, publishing appropriate events
    pub fn process_match_result(&self, result: &MatchResult) {
        // Publish trade events
        for trade in &result.trades {
            self.event_bus.publish(Event::Trade(TradeEvent::Executed(trade.clone())));
        }

        // Publish order events
        if let Some(order) = &result.processed_order {
            self.event_bus.publish(Event::Order(OrderEvent::Created(order.clone())));
        }

        // Publish affected order events
        for order in &result.affected_orders {
            self.event_bus.publish(Event::Order(OrderEvent::Modified {
                old_order: order.clone(), // Note: In a real system, we'd track the previous state
                new_order: order.clone(),
            }));
        }
    }
}

impl EventPublisher for MatchingEnginePublisher {
    fn publish_events(&self) {
        // Get current state from the engine
        let engine = self.engine.read();
        let mut last_state = self.last_state.lock();
        
        // Get the order book from the engine
        let order_book = engine.order_book();
        
        // Publish order book events for best prices
        let new_bid = order_book.best_bid();
        let new_ask = order_book.best_ask();
        
        // Only publish if we have valid prices
        if new_bid.is_some() || new_ask.is_some() {
            self.event_bus.publish(Event::OrderBook(OrderBookEvent::BestPricesChanged {
                instrument_id: engine.instrument_id(),
                old_bid: last_state.best_bid,
                new_bid,
                old_ask: last_state.best_ask,
                new_ask,
            }));
            
            // Update our last known state
            last_state.best_bid = new_bid;
            last_state.best_ask = new_ask;
        }
        
        // Clear the orders and trades vectors to avoid the dead code warning
        last_state.orders.clear();
        last_state.trades.clear();
    }
}

/// Publisher for order book events
pub struct OrderBookPublisher {
    /// Reference to the event bus
    event_bus: EventBus,
    /// The order book being monitored
    order_book: Arc<parking_lot::RwLock<OrderBook>>,
    /// Last observed state, used to detect changes
    last_state: parking_lot::Mutex<OrderBookState>,
}

/// Tracks the state of the order book for event generation
struct OrderBookState {
    /// Last known best bid price
    best_bid: Option<Decimal>,
    /// Last known best ask price
    best_ask: Option<Decimal>,
    /// Last known price levels for bids
    bid_levels: Vec<(Decimal, Decimal)>, // (price, volume)
    /// Last known price levels for asks
    ask_levels: Vec<(Decimal, Decimal)>, // (price, volume)
}

impl OrderBookPublisher {
    /// Creates a new publisher for the order book
    pub fn new(event_bus: EventBus, order_book: Arc<parking_lot::RwLock<OrderBook>>) -> Self {
        // Initialize with current state
        let best_bid;
        let best_ask;
        
        // Scope the read lock to avoid the borrowing issue
        {
            let book = order_book.read();
            best_bid = book.best_bid();
            best_ask = book.best_ask();
        }
        
        let last_state = OrderBookState {
            best_bid,
            best_ask,
            bid_levels: Vec::new(), // Would be populated from initial state
            ask_levels: Vec::new(), // Would be populated from initial state
        };

        Self {
            event_bus,
            order_book,
            last_state: parking_lot::Mutex::new(last_state),
        }
    }

    /// Checks for and publishes best price changes
    fn check_best_prices(&self) {
        let book = self.order_book.read();
        let mut state = self.last_state.lock();
        
        let new_bid = book.best_bid();
        let new_ask = book.best_ask();
        
        if new_bid != state.best_bid || new_ask != state.best_ask {
            self.event_bus.publish(Event::OrderBook(OrderBookEvent::BestPricesChanged {
                instrument_id: book.instrument_id(),
                old_bid: state.best_bid,
                new_bid,
                old_ask: state.best_ask,
                new_ask,
            }));
            
            state.best_bid = new_bid;
            state.best_ask = new_ask;
        }
    }
    
    /// Checks for and publishes price level changes
    fn check_price_levels(&self) {
        let book = self.order_book.read();
        let mut state = self.last_state.lock();
        
        // Get current state from bid side
        let mut current_bid_levels = Vec::new();
        // Track bid levels from highest price to lowest
        let mut price = book.best_bid();
        while let Some(current_price) = price {
            if let Some(volume) = book.volume_at_price(Side::Bid, current_price) {
                current_bid_levels.push((current_price, volume));
                
                // Move to next lower price (for bids we go down)
                // We'll estimate the next price by subtracting a small amount
                // This is a simplification - a real implementation would need to know all price points
                price = if current_price > Decimal::new(1, 2) { // Don't go below 0.01
                    Some(current_price - Decimal::new(1, 2)) // Subtract 0.01
                } else {
                    None
                };
            } else {
                // No volume at this price, stop searching
                break;
            }
        }
        
        // Get current state from ask side
        let mut current_ask_levels = Vec::new();
        // Track ask levels from lowest price to highest
        let mut price = book.best_ask();
        while let Some(current_price) = price {
            if let Some(volume) = book.volume_at_price(Side::Ask, current_price) {
                current_ask_levels.push((current_price, volume));
                
                // Move to next higher price (for asks we go up)
                price = Some(current_price + Decimal::new(1, 2)); // Add 0.01
            } else {
                // No volume at this price, stop searching
                break;
            }
        }
        
        // Compare current state with last state to detect changes
        
        // Check for added or updated bid levels
        for &(price, volume) in &current_bid_levels {
            let previous = state.bid_levels.iter().find(|&&(p, _)| p == price);
            
            match previous {
                Some(&(_, prev_volume)) if prev_volume != volume => {
                    // Volume changed - publish update
                    self.event_bus.publish(Event::OrderBook(OrderBookEvent::LevelUpdated {
                        instrument_id: book.instrument_id(),
                        side: Side::Bid,
                        price,
                        old_volume: prev_volume,
                        new_volume: volume,
                    }));
                }
                None => {
                    // New level - publish add
                    self.event_bus.publish(Event::OrderBook(OrderBookEvent::LevelAdded {
                        instrument_id: book.instrument_id(),
                        side: Side::Bid,
                        price,
                        volume,
                    }));
                }
                _ => {} // No change
            }
        }
        
        // Check for removed bid levels
        for &(price, _volume) in &state.bid_levels {
            if !current_bid_levels.iter().any(|&(p, _)| p == price) {
                // Level removed - publish remove
                self.event_bus.publish(Event::OrderBook(OrderBookEvent::LevelRemoved {
                    instrument_id: book.instrument_id(),
                    side: Side::Bid,
                    price,
                }));
            }
        }
        
        // Check for added or updated ask levels
        for &(price, volume) in &current_ask_levels {
            let previous = state.ask_levels.iter().find(|&&(p, _)| p == price);
            
            match previous {
                Some(&(_, prev_volume)) if prev_volume != volume => {
                    // Volume changed - publish update
                    self.event_bus.publish(Event::OrderBook(OrderBookEvent::LevelUpdated {
                        instrument_id: book.instrument_id(),
                        side: Side::Ask,
                        price,
                        old_volume: prev_volume,
                        new_volume: volume,
                    }));
                }
                None => {
                    // New level - publish add
                    self.event_bus.publish(Event::OrderBook(OrderBookEvent::LevelAdded {
                        instrument_id: book.instrument_id(),
                        side: Side::Ask,
                        price,
                        volume,
                    }));
                }
                _ => {} // No change
            }
        }
        
        // Check for removed ask levels
        for &(price, _volume) in &state.ask_levels {
            if !current_ask_levels.iter().any(|&(p, _)| p == price) {
                // Level removed - publish remove
                self.event_bus.publish(Event::OrderBook(OrderBookEvent::LevelRemoved {
                    instrument_id: book.instrument_id(),
                    side: Side::Ask,
                    price,
                }));
            }
        }
        
        // Update state with current levels
        state.bid_levels = current_bid_levels;
        state.ask_levels = current_ask_levels;
    }
}

impl EventPublisher for OrderBookPublisher {
    fn publish_events(&self) {
        self.check_best_prices();
        self.check_price_levels();
        // In a complete implementation, would also check for other state changes
    }
} 
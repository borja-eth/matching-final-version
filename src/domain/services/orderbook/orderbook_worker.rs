//--------------------------------------------------------------------------------------------------
// MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module implements a thread worker for processing order book operations asynchronously.
// It uses message passing to safely interact with the order book from multiple threads.
//
// | Component           | Description                                                 |
// |---------------------|-------------------------------------------------------------|
// | OrderBookWorker     | Worker thread managing operations on an OrderBook           |
// | OrderBookClient     | Client interface to interact with the worker                |
// | OrderBookCommand    | Commands sent to the worker                                 |
// | OrderBookResponse   | Responses sent back from the worker                         |
//
//--------------------------------------------------------------------------------------------------
// STRUCTS
//--------------------------------------------------------------------------------------------------
// | Name               | Description                                       | Key Methods         |
// |--------------------|---------------------------------------------------|---------------------|
// | OrderBookWorker    | Worker thread managing OrderBook                  | start               |
// |                    |                                                   | handle_command      |
// |--------------------|---------------------------------------------------|---------------------|
// | OrderBookClient    | Client interface to worker                        | add_order           |
// |                    |                                                   | remove_order        |
// |                    |                                                   | get_depth           |
//
//--------------------------------------------------------------------------------------------------
// ENUMS
//--------------------------------------------------------------------------------------------------
// | Name               | Description                                       | Variants            |
// |--------------------|---------------------------------------------------|---------------------|
// | OrderBookCommand   | Commands sent to worker                           | AddOrder            |
// |                    |                                                   | RemoveOrder         |
// |                    |                                                   | GetDepth            |
// |                    |                                                   | Shutdown            |
// |--------------------|---------------------------------------------------|---------------------|
// | OrderBookResponse  | Responses from worker                             | Ok                  |
// |                    |                                                   | OrderAdded          |
// |                    |                                                   | OrderRemoved        |
// |                    |                                                   | Depth               |
// |                    |                                                   | Error               |
//--------------------------------------------------------------------------------------------------

use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::domain::models::types::Order;
use crate::domain::services::orderbook::OrderbookError;
use crate::domain::services::orderbook::orderbook::{OrderBook, OrderBookError};
use crate::domain::services::orderbook::depth::{DepthSnapshot, DepthTracker};

/// Commands that can be sent to the OrderBookWorker
#[derive(Debug)]
enum OrderBookCommand {
    /// Add an order to the book
    AddOrder {
        order: Order,
        response_tx: oneshot::Sender<Result<(), OrderbookError>>,
    },
    
    /// Remove an order from the book
    RemoveOrder {
        order_id: Uuid,
        response_tx: oneshot::Sender<Result<Order, OrderbookError>>,
    },
    
    /// Get the current market depth
    GetDepth {
        limit: usize,
        response_tx: oneshot::Sender<DepthSnapshot>,
    },
    
    /// Get the best bid order
    GetBestBid {
        response_tx: oneshot::Sender<Option<Order>>,
    },
    
    /// Get the best ask order
    GetBestAsk {
        response_tx: oneshot::Sender<Option<Order>>,
    },
    
    /// Shut down the worker thread
    Shutdown,
}

/// Worker thread that processes order book operations
pub struct OrderBookWorker {
    /// The order book being managed by this worker
    order_book: OrderBook,
    
    /// Depth tracker for aggregated book views
    depth_tracker: DepthTracker,
    
    /// Command receiver
    command_rx: Mutex<Option<Receiver<OrderBookCommand>>>,
    
    /// Worker thread handle
    thread_handle: Option<JoinHandle<()>>,
}

impl OrderBookWorker {
    /// Creates a new OrderBookWorker for a specific instrument.
    ///
    /// # Arguments
    /// * `instrument_id` - The ID of the instrument this order book manages
    pub fn new(instrument_id: Uuid) -> Self {
        Self {
            order_book: OrderBook::new(instrument_id),
            depth_tracker: DepthTracker::new(instrument_id),
            command_rx: Mutex::new(None),
            thread_handle: None,
        }
    }
    
    /// Starts the worker thread and returns a client to interact with it.
    ///
    /// # Returns
    /// A client that can be used to send commands to this worker
    pub fn start(mut self) -> (OrderBookClient, JoinHandle<()>) {
        let (command_tx, command_rx) = mpsc::channel(1000);
        *self.command_rx.lock().unwrap() = Some(command_rx);
        
        let client = OrderBookClient::new(command_tx);
        
        let handle = thread::spawn(move || {
            // Tokio runtime for the worker thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for OrderBookWorker");
            
            rt.block_on(async {
                self.run().await;
            });
        });
        
        (client, handle)
    }
    
    /// Main worker loop that processes commands
    async fn run(&mut self) {
        let mut rx = self.command_rx.lock().unwrap().take().expect("Command receiver not set");
        
        while let Some(cmd) = rx.recv().await {
            match cmd {
                OrderBookCommand::Shutdown => break,
                _ => self.handle_command(cmd),
            }
        }
    }
    
    /// Processes a single command
    fn handle_command(&mut self, cmd: OrderBookCommand) {
        match cmd {
            OrderBookCommand::AddOrder { order, response_tx } => {
                let result = self.order_book.add_order(order.clone());
                if result.is_ok() {
                    // Update depth tracker on successful add
                    self.depth_tracker.update_order_added(&order);
                }
                // Convert OrderBookError to OrderbookError if needed
                let result = result.map_err(|e| match e {
                    OrderBookError::WrongInstrument { expected, got } => 
                        OrderbookError::WrongInstrument { expected, got },
                    OrderBookError::NoLimitPrice => 
                        OrderbookError::NoLimitPrice,
                    OrderBookError::OrderNotFound(id) => 
                        OrderbookError::OrderNotFound(id),
                    OrderBookError::InvalidPrice(price) => 
                        OrderbookError::InvalidPrice(price),
                    OrderBookError::InvalidQuantity(qty) => 
                        OrderbookError::InvalidQuantity(qty),
                });
                let _ = response_tx.send(result);
            },
            
            OrderBookCommand::RemoveOrder { order_id, response_tx } => {
                let result = self.order_book.remove_order(order_id);
                if let Ok(ref order) = result {
                    // Update depth tracker on successful remove
                    self.depth_tracker.update_order_removed(order);
                }
                // Convert OrderBookError to OrderbookError if needed
                let result = result.map_err(|e| match e {
                    OrderBookError::WrongInstrument { expected, got } => 
                        OrderbookError::WrongInstrument { expected, got },
                    OrderBookError::NoLimitPrice => 
                        OrderbookError::NoLimitPrice,
                    OrderBookError::OrderNotFound(id) => 
                        OrderbookError::OrderNotFound(id),
                    OrderBookError::InvalidPrice(price) => 
                        OrderbookError::InvalidPrice(price),
                    OrderBookError::InvalidQuantity(qty) => 
                        OrderbookError::InvalidQuantity(qty),
                });
                let _ = response_tx.send(result);
            },
            
            OrderBookCommand::GetDepth { limit, response_tx } => {
                let snapshot = self.depth_tracker.get_snapshot(limit);
                let _ = response_tx.send(snapshot);
            },
            
            OrderBookCommand::GetBestBid { response_tx } => {
                let best_bid = self.order_book.get_best_bid().cloned();
                let _ = response_tx.send(best_bid);
            },
            
            OrderBookCommand::GetBestAsk { response_tx } => {
                let best_ask = self.order_book.get_best_ask().cloned();
                let _ = response_tx.send(best_ask);
            },
            
            OrderBookCommand::Shutdown => {
                // Handled in the run loop
            },
        }
    }
}

/// Client interface to interact with the OrderBookWorker
#[derive(Clone)]
pub struct OrderBookClient {
    command_tx: Sender<OrderBookCommand>,
}

impl OrderBookClient {
    /// Creates a new client connected to the worker.
    ///
    /// # Arguments
    /// * `command_tx` - Sender for commands to the worker
    fn new(command_tx: Sender<OrderBookCommand>) -> Self {
        Self { command_tx }
    }
    
    /// Adds an order to the order book.
    ///
    /// # Arguments
    /// * `order` - The order to add
    ///
    /// # Returns
    /// A result indicating success or the error that occurred
    pub async fn add_order(&self, order: Order) -> Result<(), OrderbookError> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(OrderBookCommand::AddOrder {
            order,
            response_tx,
        }).await.map_err(|_| {
            OrderbookError::Internal("OrderBookWorker channel closed".to_string())
        })?;
        
        response_rx.await.map_err(|_| {
            OrderbookError::Internal("Failed to receive response from OrderBookWorker".to_string())
        })?
    }
    
    /// Removes an order from the order book.
    ///
    /// # Arguments
    /// * `order_id` - The ID of the order to remove
    ///
    /// # Returns
    /// The removed order if successful, or an error
    pub async fn remove_order(&self, order_id: Uuid) -> Result<Order, OrderbookError> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(OrderBookCommand::RemoveOrder {
            order_id,
            response_tx,
        }).await.map_err(|_| {
            OrderbookError::Internal("OrderBookWorker channel closed".to_string())
        })?;
        
        response_rx.await.map_err(|_| {
            OrderbookError::Internal("Failed to receive response from OrderBookWorker".to_string())
        })?
    }
    
    /// Gets the current market depth.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of price levels to include
    ///
    /// # Returns
    /// A snapshot of the current order book depth
    pub async fn get_depth(&self, limit: usize) -> Result<DepthSnapshot, OrderbookError> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(OrderBookCommand::GetDepth {
            limit,
            response_tx,
        }).await.map_err(|_| {
            OrderbookError::Internal("OrderBookWorker channel closed".to_string())
        })?;
        
        response_rx.await.map_err(|_| {
            OrderbookError::Internal("Failed to receive response from OrderBookWorker".to_string())
        })
    }
    
    /// Gets the best bid order.
    ///
    /// # Returns
    /// The best bid order, if one exists
    pub async fn get_best_bid(&self) -> Result<Option<Order>, OrderbookError> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(OrderBookCommand::GetBestBid {
            response_tx,
        }).await.map_err(|_| {
            OrderbookError::Internal("OrderBookWorker channel closed".to_string())
        })?;
        
        response_rx.await.map_err(|_| {
            OrderbookError::Internal("Failed to receive response from OrderBookWorker".to_string())
        })
    }
    
    /// Gets the best ask order.
    ///
    /// # Returns
    /// The best ask order, if one exists
    pub async fn get_best_ask(&self) -> Result<Option<Order>, OrderbookError> {
        let (response_tx, response_rx) = oneshot::channel();
        
        self.command_tx.send(OrderBookCommand::GetBestAsk {
            response_tx,
        }).await.map_err(|_| {
            OrderbookError::Internal("OrderBookWorker channel closed".to_string())
        })?;
        
        response_rx.await.map_err(|_| {
            OrderbookError::Internal("Failed to receive response from OrderBookWorker".to_string())
        })
    }
    
    /// Shuts down the worker thread.
    pub async fn shutdown(&self) -> Result<(), OrderbookError> {
        self.command_tx.send(OrderBookCommand::Shutdown).await.map_err(|_| {
            OrderbookError::Internal("OrderBookWorker channel closed".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::types::{OrderType, OrderStatus, TimeInForce, CreatedFrom, Side};
    use chrono::Utc;
    
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
    
    #[tokio::test]
    async fn test_add_and_get_orders() {
        let instrument_id = Uuid::new_v4();
        let worker = OrderBookWorker::new(instrument_id);
        let (client, _handle) = worker.start();
        
        // Create and add a bid order
        let bid_order = create_test_order(Side::Bid, 100_000, 5_000, instrument_id);
        client.add_order(bid_order.clone()).await.expect("Failed to add bid order");
        
        // Create and add an ask order
        let ask_order = create_test_order(Side::Ask, 101_000, 3_000, instrument_id);
        client.add_order(ask_order.clone()).await.expect("Failed to add ask order");
        
        // Verify best bid
        let best_bid = client.get_best_bid().await.expect("Failed to get best bid");
        assert!(best_bid.is_some());
        assert_eq!(best_bid.unwrap().id, bid_order.id);
        
        // Verify best ask
        let best_ask = client.get_best_ask().await.expect("Failed to get best ask");
        assert!(best_ask.is_some());
        assert_eq!(best_ask.unwrap().id, ask_order.id);
        
        // Get depth snapshot
        let depth = client.get_depth(10).await.expect("Failed to get depth");
        assert_eq!(depth.instrument_id, instrument_id);
        assert_eq!(depth.bids.len(), 1);
        assert_eq!(depth.asks.len(), 1);
        
        // Clean up
        client.shutdown().await.expect("Failed to shut down worker");
    }
    
    #[tokio::test]
    async fn test_remove_order() {
        let instrument_id = Uuid::new_v4();
        let worker = OrderBookWorker::new(instrument_id);
        let (client, _handle) = worker.start();
        
        // Create and add a bid order
        let bid_order = create_test_order(Side::Bid, 100_000, 5_000, instrument_id);
        client.add_order(bid_order.clone()).await.expect("Failed to add bid order");
        
        // Remove the order
        let removed = client.remove_order(bid_order.id).await.expect("Failed to remove order");
        assert_eq!(removed.id, bid_order.id);
        
        // Verify best bid is gone
        let best_bid = client.get_best_bid().await.expect("Failed to get best bid");
        assert!(best_bid.is_none());
        
        // Clean up
        client.shutdown().await.expect("Failed to shut down worker");
    }
    
    #[tokio::test]
    async fn test_multiple_price_levels() {
        let instrument_id = Uuid::new_v4();
        let worker = OrderBookWorker::new(instrument_id);
        let (client, _handle) = worker.start();
        
        // Add orders at different price levels
        let prices = [100_000, 101_000, 99_000];
        for price in prices {
            let bid_order = create_test_order(Side::Bid, price, 1_000, instrument_id);
            client.add_order(bid_order).await.expect("Failed to add bid order");
        }
        
        // Get depth snapshot
        let depth = client.get_depth(10).await.expect("Failed to get depth");
        assert_eq!(depth.bids.len(), 3);
        
        // Verify best bid is highest price
        let best_bid = client.get_best_bid().await.expect("Failed to get best bid").unwrap();
        assert_eq!(best_bid.limit_price, Some(101_000));
        
        // Clean up
        client.shutdown().await.expect("Failed to shut down worker");
    }
} 
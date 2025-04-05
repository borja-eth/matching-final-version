//--------------------------------------------------------------------------------------------------
// TEST MODULE OVERVIEW
//--------------------------------------------------------------------------------------------------
// This module contains integration tests for the API.
// It tests all endpoints and verifies the responses.
//--------------------------------------------------------------------------------------------------

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use hyper::Response;
use serde_json::{json, Value, from_slice};
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

use ultimate_matching::{
    Api, 
    api::AppState,
    events::EventBus,
};

/// Sets up a test router with app state.
/// Returns the router and a predefined instrument ID.
async fn setup_test_router() -> (Router, Uuid) {
    // Create a test instrument ID
    let instrument_id = Uuid::new_v4();
    
    // Set up the event system for testing
    let event_bus = EventBus::default();
    
    // Create API
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let api = Api::new(addr, event_bus.clone());
    
    // Get the router with state attached
    let app = api.routes();
    
    // Add test instrument to the API's app state
    let state = Arc::new(AppState::new(event_bus));
    state.add_instrument(instrument_id).await;
    
    (app, instrument_id)
}

/// Helper to parse JSON responses
async fn parse_json_response(response: Response<Body>) -> Value {
    // Convert the response body to bytes
    let body_bytes = to_bytes(response.into_body(), 1024 * 1024) // 1MB limit
        .await
        .unwrap();
    
    // Parse the JSON from the bytes
    from_slice(&body_bytes).unwrap()
}

#[tokio::test]
async fn test_health_endpoint() {
    // Setup
    let (app, _) = setup_test_router().await;
    
    // Execute
    let response = app
        .clone()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    // Verify
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_order() {
    // Setup
    let (app, instrument_id) = setup_test_router().await;
    
    // Prepare JSON payload
    let json_body = json!({
        "ext_id": "test-order-1",
        "account_id": Uuid::new_v4().to_string(),
        "order_type": "Limit",
        "instrument_id": instrument_id.to_string(),
        "side": "Bid",
        "limit_price": "100.50",
        "base_amount": "1.5",
        "time_in_force": "GTC"
    });
    
    // Execute - create a limit buy order
    let response = app
        .clone()
        .oneshot(
            Request::post("/orders")
                .header("Content-Type", "application/json")
                .body(Body::from(json_body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Verify
    assert_eq!(response.status(), StatusCode::CREATED);
    
    // Parse response JSON
    let body = parse_json_response(response).await;
    
    // Check fields
    assert_eq!(body["ext_id"], "test-order-1");
    assert_eq!(body["order_type"], "Limit");
    assert_eq!(body["side"], "Bid");
    assert_eq!(body["limit_price"], "100.50");
    assert_eq!(body["base_amount"], "1.5");
}

#[tokio::test]
async fn test_cancel_order() {
    // Setup
    let (app, instrument_id) = setup_test_router().await;
    
    // First create an order
    let json_body = json!({
        "ext_id": "test-order-2",
        "account_id": Uuid::new_v4().to_string(),
        "order_type": "Limit",
        "instrument_id": instrument_id.to_string(),
        "side": "Bid",
        "limit_price": "100.50",
        "base_amount": "1.5",
        "time_in_force": "GTC"
    });
    
    let create_response = app
        .clone()
        .oneshot(
            Request::post("/orders")
                .header("Content-Type", "application/json")
                .body(Body::from(json_body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let body = parse_json_response(create_response).await;
    let order_id = body["id"].as_str().unwrap();
    
    // Execute - cancel the order
    let url = format!("/orders/{}?instrument_id={}", order_id, instrument_id.to_string());
    let cancel_response = app
        .clone()
        .oneshot(
            Request::delete(&url)
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    // Verify
    assert_eq!(cancel_response.status(), StatusCode::OK);
    
    // Parse response JSON
    let body = parse_json_response(cancel_response).await;
    
    // Check fields
    assert_eq!(body["id"], order_id);
    assert_eq!(body["status"], "Cancelled");
}

#[tokio::test]
async fn test_get_orderbook() {
    // Setup
    let (app, instrument_id) = setup_test_router().await;
    
    // Add some orders to create an orderbook
    // Bid order
    let bid_json = json!({
        "ext_id": "test-bid-1",
        "account_id": Uuid::new_v4().to_string(),
        "order_type": "Limit",
        "instrument_id": instrument_id.to_string(),
        "side": "Bid",
        "limit_price": "100.00",
        "base_amount": "1.0",
        "time_in_force": "GTC"
    });
    
    app.clone()
        .oneshot(
            Request::post("/orders")
                .header("Content-Type", "application/json")
                .body(Body::from(bid_json.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Ask order
    let ask_json = json!({
        "ext_id": "test-ask-1",
        "account_id": Uuid::new_v4().to_string(),
        "order_type": "Limit",
        "instrument_id": instrument_id.to_string(),
        "side": "Ask",
        "limit_price": "101.00",
        "base_amount": "1.0",
        "time_in_force": "GTC"
    });
    
    app.clone()
        .oneshot(
            Request::post("/orders")
                .header("Content-Type", "application/json")
                .body(Body::from(ask_json.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Execute - get the orderbook
    let url = format!("/instruments/{}/orderbook", instrument_id);
    let response = app
        .clone()
        .oneshot(
            Request::get(&url)
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    // Verify
    assert_eq!(response.status(), StatusCode::OK);
    
    // Parse response JSON
    let body = parse_json_response(response).await;
    
    // Check structure
    assert!(body["bids"].is_array());
    assert!(body["asks"].is_array());
    assert!(body["timestamp"].is_string());
    assert_eq!(body["instrument_id"], instrument_id.to_string());
    
    // Check content (assuming at least one bid and one ask)
    assert!(!body["bids"].as_array().unwrap().is_empty());
    assert!(!body["asks"].as_array().unwrap().is_empty());
    
    // Check price levels
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    
    if !bids.is_empty() {
        assert_eq!(bids[0]["price"], "100.00");
        assert_eq!(bids[0]["volume"], "1.0");
    }
    
    if !asks.is_empty() {
        assert_eq!(asks[0]["price"], "101.00");
        assert_eq!(asks[0]["volume"], "1.0");
    }
}

#[tokio::test]
async fn test_get_depth() {
    // Setup
    let (app, instrument_id) = setup_test_router().await;
    
    // Add some orders to create depth
    // Bid order
    let bid_json = json!({
        "ext_id": "test-bid-2",
        "account_id": Uuid::new_v4().to_string(),
        "order_type": "Limit",
        "instrument_id": instrument_id.to_string(),
        "side": "Bid",
        "limit_price": "99.00",
        "base_amount": "2.0",
        "time_in_force": "GTC"
    });
    
    app.clone()
        .oneshot(
            Request::post("/orders")
                .header("Content-Type", "application/json")
                .body(Body::from(bid_json.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Ask order
    let ask_json = json!({
        "ext_id": "test-ask-2",
        "account_id": Uuid::new_v4().to_string(),
        "order_type": "Limit",
        "instrument_id": instrument_id.to_string(),
        "side": "Ask",
        "limit_price": "102.00",
        "base_amount": "2.0",
        "time_in_force": "GTC"
    });
    
    app.clone()
        .oneshot(
            Request::post("/orders")
                .header("Content-Type", "application/json")
                .body(Body::from(ask_json.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Execute - get the depth with level parameter
    let url = format!("/instruments/{}/depth?level=5", instrument_id);
    let response = app
        .clone()
        .oneshot(
            Request::get(&url)
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    // Verify
    assert_eq!(response.status(), StatusCode::OK);
    
    // Parse response JSON
    let body = parse_json_response(response).await;
    
    // Check structure (same as orderbook)
    assert!(body["bids"].is_array());
    assert!(body["asks"].is_array());
    assert!(body["timestamp"].is_string());
    assert_eq!(body["instrument_id"], instrument_id.to_string());
}

#[tokio::test]
async fn test_create_instrument() {
    // Setup
    let (app, _) = setup_test_router().await;
    
    // Execute - create a new instrument
    let json_body = json!({
        "name": "BTC/USD",
        "base_currency": "BTC",
        "quote_currency": "USD"
    });
    
    let response = app
        .clone()
        .oneshot(
            Request::post("/instruments")
                .header("Content-Type", "application/json")
                .body(Body::from(json_body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    // Verify
    assert_eq!(response.status(), StatusCode::CREATED);
    
    // Parse response JSON
    let body = parse_json_response(response).await;
    
    // Check fields
    assert_eq!(body["name"], "BTC/USD");
    assert_eq!(body["base_currency"], "BTC");
    assert_eq!(body["quote_currency"], "USD");
    assert!(Uuid::parse_str(body["id"].as_str().unwrap()).is_ok());
}

#[tokio::test]
async fn test_list_instruments() {
    // Setup
    let (app, instrument_id) = setup_test_router().await;
    
    // Create another instrument
    let json_body = json!({
        "name": "ETH/USD",
        "base_currency": "ETH",
        "quote_currency": "USD"
    });
    
    let create_response = app
        .clone()
        .oneshot(
            Request::post("/instruments")
                .header("Content-Type", "application/json")
                .body(Body::from(json_body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(create_response.status(), StatusCode::CREATED);
    
    // Execute - list all instruments
    let response = app
        .clone()
        .oneshot(
            Request::get("/instruments")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    
    // Verify
    assert_eq!(response.status(), StatusCode::OK);
    
    // Parse response JSON
    let body = parse_json_response(response).await;
    
    // Check that it's an array and has at least our initial instrument
    assert!(body.is_array());
    let instruments = body.as_array().unwrap();
    assert!(instruments.len() >= 1);
    
    // Check if our original instrument ID is in the list
    let has_original_instrument = instruments.iter()
        .any(|v| v.as_str().unwrap() == instrument_id.to_string());
    
    assert!(has_original_instrument);
} 
//--------------------------------------------------------------------------------------------------
// FUNCTIONS
//--------------------------------------------------------------------------------------------------
// | Name                  | Description                            | Return Type         |
// |-----------------------|----------------------------------------|---------------------|
// | health                | Health check endpoint                  | Response            |
// | create_order          | Create and process a new order         | ApiResult<Response> |
// | cancel_order          | Cancel an existing order               | ApiResult<Response> |
// | get_order             | Get details of an order                | ApiResult<Response> |
// | get_orderbook         | Get current orderbook for instrument   | ApiResult<Response> |
// | get_depth             | Get market depth for instrument        | ApiResult<Response> |
// | get_trades            | Get recent trades for instrument       | ApiResult<Response> |
// | create_instrument     | Create a new trading instrument        | ApiResult<Response> |
// | list_instruments      | List all available instruments         | ApiResult<Response> |
//--------------------------------------------------------------------------------------------------

use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    extract::{Path, Extension, Query},
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use super::{
    AppState, 
    ApiError, 
    ApiResult, 
    CreateOrderRequest, 
    OrderResponse,
    CreateInstrumentRequest,
    InstrumentResponse,
    TradeResponse,
    DepthResponse,
};
use crate::types::TimeInForce;

/// Health check endpoint
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok"
    }))
}

/// Create and process a new order
pub async fn create_order(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<CreateOrderRequest>,
) -> ApiResult<Response> {
    // Get the matching engine for the instrument
    let engine_lock = state.get_engine(&req.instrument_id).await
        .ok_or_else(|| ApiError::NotFound(format!("Instrument {} not found", req.instrument_id)))?;
    
    // Convert request to order
    let order = req.into_order();
    
    // Use if/else instead of ternary operator
    let time_in_force = if order.expiration_date.timestamp() > chrono::Utc::now().timestamp() + 86400 {
        TimeInForce::GTC
    } else {
        TimeInForce::IOC
    };
    
    // Process the order
    let mut engine = engine_lock.write().await;
    let result = engine.process_order(order.clone(), time_in_force)?;
    
    // Return the processed order
    if let Some(processed_order) = result.processed_order {
        let response = OrderResponse::from(processed_order);
        Ok((StatusCode::CREATED, Json(response)).into_response())
    } else {
        Err(ApiError::Internal("Order processing failed".to_string()))
    }
}

/// Cancel an existing order
pub async fn cancel_order(
    Extension(state): Extension<Arc<AppState>>,
    Path(order_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Response> {
    // Get instrument ID from query params
    let instrument_id = params.get("instrument_id")
        .and_then(|id_str| Uuid::parse_str(id_str).ok())
        .ok_or_else(|| ApiError::BadRequest("instrument_id query parameter is required".to_string()))?;
    
    // Get the matching engine for the instrument
    let engine_lock = state.get_engine(&instrument_id).await
        .ok_or_else(|| ApiError::NotFound(format!("Instrument {} not found", instrument_id)))?;
    
    // Cancel the order
    let mut engine = engine_lock.write().await;
    let order = engine.cancel_order(order_id)?;
    
    // Return the cancelled order
    let response = OrderResponse::from(order);
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Get details of an order
pub async fn get_order(
    Extension(state): Extension<Arc<AppState>>,
    Path(order_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Response> {
    // Get instrument ID from query params
    let instrument_id = params.get("instrument_id")
        .and_then(|id_str| Uuid::parse_str(id_str).ok())
        .ok_or_else(|| ApiError::BadRequest("instrument_id query parameter is required".to_string()))?;
    
    // Get the matching engine for the instrument
    let engine_lock = state.get_engine(&instrument_id).await
        .ok_or_else(|| ApiError::NotFound(format!("Instrument {} not found", instrument_id)))?;
    
    // Get the order from the engine state (this would need to be implemented in the engine)
    let _matching_engine = engine_lock.read().await;
    
    // This is a placeholder - the actual implementation would depend on how orders are stored
    // We would need to add a method to retrieve an order by ID from the engine
    // Assuming the engine has a method get_order(order_id: Uuid) -> Option<Order>
    
    // For demonstration, we'll return a not found error
    Err(ApiError::NotFound(format!("Order {} not found", order_id)))
}

/// Get current orderbook for an instrument
pub async fn get_orderbook(
    Extension(state): Extension<Arc<AppState>>,
    Path(instrument_id): Path<Uuid>,
) -> ApiResult<Response> {
    // Get the matching engine for the instrument
    let engine_lock = state.get_engine(&instrument_id).await
        .ok_or_else(|| ApiError::NotFound(format!("Instrument {} not found", instrument_id)))?;
    
    // Since get_depth requires mutable access, we need to use a write guard
    let mut engine = engine_lock.write().await;
    let depth = engine.get_depth(20);
    let response = DepthResponse::from(depth);
    
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Get market depth for an instrument
pub async fn get_depth(
    Extension(state): Extension<Arc<AppState>>,
    Path(instrument_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Response> {
    // Get depth level from query params (default 10)
    let level = params.get("level")
        .and_then(|level_str| level_str.parse::<usize>().ok())
        .unwrap_or(10);
    
    // Get the matching engine for the instrument
    let engine_lock = state.get_engine(&instrument_id).await
        .ok_or_else(|| ApiError::NotFound(format!("Instrument {} not found", instrument_id)))?;
    
    // Since get_depth requires mutable access, we need to use a write guard
    let mut engine = engine_lock.write().await;
    let depth = engine.get_depth(level);
    let response = DepthResponse::from(depth);
    
    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Get recent trades for an instrument
pub async fn get_trades(
    Extension(state): Extension<Arc<AppState>>,
    Path(instrument_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Response> {
    // Get limit from query params (default 20)
    let _limit = params.get("limit")
        .and_then(|limit_str| limit_str.parse::<usize>().ok())
        .unwrap_or(20);
    
    // Get the matching engine for the instrument
    let _engine_lock = state.get_engine(&instrument_id).await
        .ok_or_else(|| ApiError::NotFound(format!("Instrument {} not found", instrument_id)))?;
    
    // This functionality would need to be added to the engine
    // For demonstration, we'll return an empty trades list
    let trades = Vec::<TradeResponse>::new();
    
    Ok((StatusCode::OK, Json(trades)).into_response())
}

/// Create a new trading instrument
pub async fn create_instrument(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<CreateInstrumentRequest>,
) -> ApiResult<Response> {
    // Generate ID if not provided
    let id = req.id.unwrap_or_else(Uuid::new_v4);
    
    // Create the instrument
    state.add_instrument(id).await;
    
    // Return the created instrument
    let response = InstrumentResponse {
        id,
        name: req.name,
        base_currency: req.base_currency,
        quote_currency: req.quote_currency,
    };
    
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// List all available instruments
pub async fn list_instruments(
    Extension(state): Extension<Arc<AppState>>,
) -> ApiResult<Response> {
    // Get all instrument IDs
    let engines = state.engines.read().await;
    let instrument_ids: Vec<Uuid> = engines.keys().cloned().collect();
    
    // Create responses with default names based on the IDs
    let instruments: Vec<InstrumentResponse> = instrument_ids
        .into_iter()
        .map(|id| InstrumentResponse {
            id,
            name: format!("BTC/USD"), // Default name
            base_currency: "BTC".to_string(),
            quote_currency: "USD".to_string(),
        })
        .collect();
    
    Ok((StatusCode::OK, Json(instruments)).into_response())
} 
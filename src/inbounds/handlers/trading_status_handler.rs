use std::sync::Arc;
use tracing::{info, debug, error};

use crate::domain::services::orderbook_manager::OrderbookManagerService;
use crate::inbounds::api_error::ApiError;
use crate::inbounds::dtos::TradingStatusRequest;

/// +----------------------------------------------------------+
/// | STRUCTS | TRAITS | ENUMS | FUNCTIONS                     |
/// +----------+-------+-------+------------------------------+
/// | Functions:                                               |
/// |   - handle_trading_status_request                        |
/// +----------------------------------------------------------+

/// Processes a trading status request from a binary payload.
///
/// # Arguments
///
/// * `request` - The raw binary payload containing the trading status request
/// * `orderbook_manager` - The service responsible for managing the orderbook
///
/// # Flow
///
/// 1. Deserializes the binary payload into a `TradingStatusRequest`
/// 2. Logs the trading status request attempt
/// 3. Requests the orderbook manager to publish the status for the specified instrument
/// 4. Handles and logs any errors that occur
///
/// # Error Handling
///
/// * Deserialization errors are converted to `ApiError::BadRequest`
/// * Orderbook manager errors are logged and converted to `ApiError`
pub fn handle_trading_status_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) -> Result<(), ApiError> {
    info!("Received trading status request");
    
    let trading_status_request: TradingStatusRequest = serde_json::from_slice(&request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid trading status request: {}", e)))?;

    debug!("Processing trading status request for instrument: {:?}", trading_status_request.instrument);
    
    orderbook_manager
        .publish_orderbook_status(trading_status_request.instrument)
        .map_err(|e| {
            error!("Error publishing orderbook status: {}", e);
            ApiError::from(e)
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn should_call_publish_status_with_correct_instrument() {
        // Test implementation would go here
    }
}

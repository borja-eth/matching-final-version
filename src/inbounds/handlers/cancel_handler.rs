use std::sync::Arc;
use tracing::{info, error};

use crate::domain::services::orderbook_manager:: OrderbookManagerService;
use crate::inbounds::dtos::CancelOrderRequest;

use super::super::api_error::ApiError;

/// +----------------------------------------------------------+
/// | STRUCTS | TRAITS | ENUMS | FUNCTIONS                     |
/// +----------+-------+-------+------------------------------+
/// | Functions:                                               |
/// |   - handle_cancel_request                                |
/// +----------------------------------------------------------+

/// Processes an order cancellation request from a binary payload.
///
/// # Arguments
///
/// * `request` - The raw binary payload containing the cancel order request
/// * `orderbook_manager` - The service responsible for managing the orderbook
///
/// # Flow
///
/// 1. Deserializes the binary payload into a `CancelOrderRequest`
/// 2. Logs the cancellation attempt with order ID
/// 3. Forwards the request to the orderbook manager
/// 4. Handles and logs any errors that occur
///
/// # Error Handling
///
/// * Deserialization errors are converted to `ApiError::BadRequest`
/// * Orderbook manager errors are logged and converted to `ApiError`
pub fn handle_cancel_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) -> Result<(), ApiError> {
    let cancel_request: CancelOrderRequest = serde_json::from_slice(&request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid cancel request: {}", e)))?;

    info!("Cancelling order: {:?}", cancel_request.order_id);
    
    orderbook_manager
        .cancel_order(&cancel_request.instrument, cancel_request.order_id)
        .map_err(|e| {
            error!("Error canceling order: {e:?}");
            ApiError::from(e)
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn should_call_orderbook_manager_with_correct_args() {
        // Test implementation would go here
    }
}

use std::sync::Arc;

use crate::domain::{
    models::types::Order,
    services::orderbook_manager::OrderbookManagerService,
};
use tracing::{info, error};

use super::super::{api_error::ApiError, dtos::PlaceOrderRequest};

/// +----------------------------------------------------------+
/// | STRUCTS | TRAITS | ENUMS | FUNCTIONS                     |
/// +----------+-------+-------+------------------------------+
/// | Functions:                                               |
/// |   - handle_place_request                                 |
/// +----------------------------------------------------------+

/// Processes an order placement request from a binary payload.
///
/// # Arguments
///
/// * `request` - The raw binary payload containing the place order request
/// * `orderbook_manager` - The service responsible for managing the orderbook
///
/// # Flow
///
/// 1. Deserializes the binary payload into a `PlaceOrderRequest`
/// 2. Logs the order placement attempt with order ID
/// 3. Converts the request DTO to a domain Order model
/// 4. Forwards the order to the orderbook manager
/// 5. Handles and logs any errors that occur
///
/// # Error Handling
///
/// * Deserialization errors are converted to `ApiError::BadRequest`
/// * Orderbook manager errors are logged and converted to `ApiError`
pub fn handle_place_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) -> Result<(), ApiError> {
    let place_request: PlaceOrderRequest = serde_json::from_slice(&request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid place request: {}", e)))?;

    info!("Placing order: {:?}", place_request.new_order_id);
    
    // Convert DTO to domain model
    let order = Order::from(place_request);
    
    // Forward to orderbook manager
    orderbook_manager
        .add_order(order)
        .map_err(|e| {
            error!("Error placing order: {e:?}");
            ApiError::from(e)
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn should_convert_dto_to_domain_model_correctly() {
        // Test implementation would go here
    }
}

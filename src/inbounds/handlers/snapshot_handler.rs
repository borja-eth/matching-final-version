use std::sync::Arc;
use tracing::{info, debug, error};

use crate::domain::services::orderbook_manager::OrderbookManagerService;
use crate::inbounds::api_error::ApiError;
use crate::inbounds::dtos::SnapshotRequest;

/// +----------------------------------------------------------+
/// | STRUCTS | TRAITS | ENUMS | FUNCTIONS                     |
/// +----------+-------+-------+------------------------------+
/// | Functions:                                               |
/// |   - handle_snapshot_request                              |
/// +----------------------------------------------------------+

/// Processes an orderbook snapshot request from a binary payload.
///
/// # Arguments
///
/// * `request` - The raw binary payload containing the snapshot request
/// * `orderbook_manager` - The service responsible for managing the orderbook
///
/// # Flow
///
/// 1. Deserializes the binary payload into a `SnapshotRequest`
/// 2. Logs the snapshot request attempt
/// 3. Requests the orderbook manager to publish a snapshot for the specified instrument
/// 4. Handles and logs any errors that occur
///
/// # Error Handling
///
/// * Deserialization errors are converted to `ApiError::BadRequest`
/// * Orderbook manager errors are logged and converted to `ApiError`
pub fn handle_snapshot_request(
    request: Vec<u8>,
    orderbook_manager: Arc<dyn OrderbookManagerService>,
) -> Result<(), ApiError> {
    info!("Received snapshot request");
    
    let snapshot_request: SnapshotRequest = serde_json::from_slice(&request)
        .map_err(|e| ApiError::BadRequest(format!("Invalid snapshot request: {}", e)))?;

    debug!("Processing snapshot request for instrument: {:?}", snapshot_request.instrument);
    
    orderbook_manager
        .publish_orderbook_snapshot(snapshot_request.instrument)
        .map_err(|e| {
            error!("Error publishing orderbook snapshot: {}", e);
            ApiError::from(e)
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn should_call_publish_snapshot_with_correct_instrument() {
        // Test implementation would go here
    }
}

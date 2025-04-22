/// +----------------------------------------------------------+
/// | MODULES                                                  |
/// +----------+-------+-------+------------------------------+
/// | Exports:                                                 |
/// |   - api_error                                            |
/// |   - dtos                                                 |
/// |   - handlers                                             |
/// +----------------------------------------------------------+

/// Error types for the inbound API layer.
pub mod api_error;

/// Data transfer objects for API requests and responses.
pub mod dtos;

/// Request handlers for inbound messages.
pub mod handlers;

pub mod billing;
pub mod identify;
pub mod prove;
pub mod routes;

pub use billing::BillingState;
pub use routes::AppState;

/// OpenAPI 3.0 canonical spec (identify · prove · usage)
pub const OPENAPI_YAML: &str = include_str!("../openapi.yaml");

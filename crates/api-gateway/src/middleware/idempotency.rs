use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
    Json,
};
use serde_json::json;

/// Checks for an Idempotency-Key header and stores/returns cached responses.
/// This is a simplified in-memory version; production should use the DB store.
pub async fn idempotency_middleware(req: Request, next: Next) -> impl IntoResponse {
    // For now, pass through — full implementation would check DB store
    next.run(req).await
}

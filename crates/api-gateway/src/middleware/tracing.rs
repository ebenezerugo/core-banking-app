use axum::{extract::Request, middleware::Next, response::IntoResponse};
use tracing::Span;
use uuid::Uuid;

/// Injects a correlation ID into each request span.
pub async fn tracing_middleware(mut req: Request, next: Next) -> impl IntoResponse {
    let correlation_id = req
        .headers()
        .get("X-Correlation-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Insert correlation ID into request extensions
    req.extensions_mut().insert(CorrelationId(correlation_id.clone()));

    let span = tracing::info_span!("http_request", correlation_id = %correlation_id);
    let _enter = span.enter();

    next.run(req).await
}

#[derive(Clone, Debug)]
pub struct CorrelationId(pub String);

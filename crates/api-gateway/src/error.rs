use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use shared::errors::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("Internal server error: {0}")]
    Internal(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Not found: {0}")]
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Domain(DomainError::NotFound(m)) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Domain(DomainError::AccountNotFound(m)) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Domain(DomainError::CustomerNotFound(m)) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Domain(DomainError::LoanNotFound(m)) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Domain(DomainError::TransactionNotFound(m)) => {
                (StatusCode::NOT_FOUND, m.clone())
            }
            AppError::Domain(DomainError::Unauthorized(m)) => {
                (StatusCode::FORBIDDEN, m.clone())
            }
            AppError::Domain(DomainError::InsufficientFunds { available, required }) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Insufficient funds: available={}, required={}", available, required),
            ),
            AppError::Domain(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

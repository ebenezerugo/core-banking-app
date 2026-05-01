use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use domain::transaction::TransactionType;
use shared::types::{AccountId, Currency, TenantId, TransactionId};

use crate::engine::{ExecuteTransactionCommand, TransactionEngine};

pub type TransactionEngineState = Arc<TransactionEngine>;

#[derive(Debug, Deserialize)]
pub struct ExecuteTransactionRequest {
    pub tenant_id: Uuid,
    pub idempotency_key: String,
    pub transaction_type: TransactionType,
    pub from_account_id: Option<Uuid>,
    pub to_account_id: Option<Uuid>,
    pub amount: Decimal,
    pub currency: Currency,
    pub fee: Option<Decimal>,
    pub description: String,
    pub reference: String,
}

#[derive(Debug, Deserialize)]
pub struct ReverseRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn execute_transaction(
    State(engine): State<TransactionEngineState>,
    Json(req): Json<ExecuteTransactionRequest>,
) -> impl IntoResponse {
    let cmd = ExecuteTransactionCommand {
        tenant_id: TenantId(req.tenant_id),
        idempotency_key: req.idempotency_key,
        transaction_type: req.transaction_type,
        from_account_id: req.from_account_id.map(AccountId),
        to_account_id: req.to_account_id.map(AccountId),
        amount: req.amount,
        currency: req.currency,
        fee: req.fee.unwrap_or(Decimal::ZERO),
        description: req.description,
        reference: req.reference,
    };
    match engine.execute(cmd).await {
        Ok(tx) => (StatusCode::CREATED, Json(serde_json::json!(tx))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn reverse_transaction(
    State(engine): State<TransactionEngineState>,
    Path((tenant_id, tx_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ReverseRequest>,
) -> impl IntoResponse {
    match engine
        .reverse(TransactionId(tx_id), TenantId(tenant_id), req.reason)
        .await
    {
        Ok(tx) => (StatusCode::CREATED, Json(serde_json::json!(tx))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

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

use shared::types::TenantId;

use crate::repository::PostgresChargeRepository;
use crate::service::ChargeService;

pub type ChargeServiceState = Arc<ChargeService<PostgresChargeRepository>>;

#[derive(Debug, Deserialize)]
pub struct ApplyChargeRequest {
    pub tenant_id: Uuid,
    pub base_amount: Decimal,
    pub account_id: Option<Uuid>,
    pub loan_id: Option<Uuid>,
    pub transaction_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct WaiveRequest {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn apply_charge(
    State(svc): State<ChargeServiceState>,
    Path((tenant_id, definition_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ApplyChargeRequest>,
) -> impl IntoResponse {
    use shared::types::{AccountId, LoanId, TransactionId};
    match svc
        .apply_charge(
            definition_id,
            TenantId(tenant_id),
            req.base_amount,
            req.account_id.map(AccountId),
            req.loan_id.map(LoanId),
            req.transaction_id.map(TransactionId),
        )
        .await
    {
        Ok(c) => (StatusCode::CREATED, Json(serde_json::json!(c))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn waive_charge(
    State(svc): State<ChargeServiceState>,
    Path(charge_id): Path<Uuid>,
    Json(req): Json<WaiveRequest>,
) -> impl IntoResponse {
    match svc.waive_charge(charge_id, req.reason).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

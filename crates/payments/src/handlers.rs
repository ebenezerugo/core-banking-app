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

use domain::payment::PaymentChannel;
use shared::types::{Currency, TenantId, TransactionId};

use crate::repository::PostgresPaymentRepository;
use crate::service::PaymentService;

pub type PaymentServiceState = Arc<PaymentService<PostgresPaymentRepository>>;

#[derive(Debug, Deserialize)]
pub struct InitiatePaymentRequest {
    pub tenant_id: Uuid,
    pub transaction_id: Uuid,
    pub channel: PaymentChannel,
    pub sender_account: String,
    pub receiver_account: String,
    pub amount: Decimal,
    pub currency: Currency,
    pub routing_number: Option<String>,
    pub swift_code: Option<String>,
    pub reference: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn initiate_payment(
    State(svc): State<PaymentServiceState>,
    Json(req): Json<InitiatePaymentRequest>,
) -> impl IntoResponse {
    match svc
        .initiate_payment(
            TenantId(req.tenant_id),
            TransactionId(req.transaction_id),
            req.channel,
            req.sender_account,
            req.receiver_account,
            req.amount,
            req.currency,
            req.routing_number,
            req.swift_code,
            req.reference,
        )
        .await
    {
        Ok(p) => (StatusCode::CREATED, Json(serde_json::json!(p))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn settle_payment(
    State(svc): State<PaymentServiceState>,
    Path((tenant_id, payment_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match svc.settle_payment(payment_id, TenantId(tenant_id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn get_payment(
    State(svc): State<PaymentServiceState>,
    Path((tenant_id, payment_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match svc.get_payment(payment_id, TenantId(tenant_id)).await {
        Ok(p) => (StatusCode::OK, Json(serde_json::json!(p))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

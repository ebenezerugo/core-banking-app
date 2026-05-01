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

use domain::account::AccountType;
use shared::types::{AccountId, Currency, CustomerId, TenantId};

use crate::repository::PostgresAccountRepository;
use crate::service::AccountService;

pub type AccountServiceState = Arc<AccountService<PostgresAccountRepository>>;

#[derive(Debug, Deserialize)]
pub struct OpenAccountRequest {
    pub tenant_id: Uuid,
    pub customer_id: Uuid,
    pub account_type: AccountType,
    pub currency: Currency,
    pub interest_rate: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct TransactionRequest {
    pub amount: Decimal,
    pub reference: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn open_account(
    State(svc): State<AccountServiceState>,
    Json(req): Json<OpenAccountRequest>,
) -> impl IntoResponse {
    match svc
        .open_account(
            TenantId(req.tenant_id),
            CustomerId(req.customer_id),
            req.account_type,
            req.currency,
            req.interest_rate,
        )
        .await
    {
        Ok(a) => (StatusCode::CREATED, Json(serde_json::json!(a))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn get_account(
    State(svc): State<AccountServiceState>,
    Path((tenant_id, account_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match svc
        .get_account(AccountId(account_id), TenantId(tenant_id))
        .await
    {
        Ok(a) => (StatusCode::OK, Json(serde_json::json!(a))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn deposit(
    State(svc): State<AccountServiceState>,
    Path((tenant_id, account_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<TransactionRequest>,
) -> impl IntoResponse {
    match svc
        .deposit(AccountId(account_id), TenantId(tenant_id), req.amount, req.reference)
        .await
    {
        Ok(a) => (StatusCode::OK, Json(serde_json::json!(a))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn withdraw(
    State(svc): State<AccountServiceState>,
    Path((tenant_id, account_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<TransactionRequest>,
) -> impl IntoResponse {
    match svc
        .withdraw(AccountId(account_id), TenantId(tenant_id), req.amount, req.reference)
        .await
    {
        Ok(a) => (StatusCode::OK, Json(serde_json::json!(a))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

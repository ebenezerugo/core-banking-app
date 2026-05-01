use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use domain::loan::{InterestType, RepaymentScheduleType};
use rust_decimal::Decimal;
use shared::types::{AccountId, CustomerId, LoanId, TenantId};
use chrono::NaiveDate;

use crate::commands::{
    ApproveLoanCommand, CreateLoanApplicationCommand, DisburseLoanCommand,
    ProcessRepaymentCommand,
};
use crate::repository::PostgresLoanRepository;
use crate::service::LoanService;

pub type LoanServiceState = Arc<LoanService<PostgresLoanRepository>>;

#[derive(Debug, Deserialize)]
pub struct CreateLoanRequest {
    pub tenant_id: Uuid,
    pub customer_id: Uuid,
    pub account_id: Uuid,
    pub principal: Decimal,
    pub interest_rate: Decimal,
    pub interest_type: InterestType,
    pub term_months: i32,
    pub repayment_schedule_type: RepaymentScheduleType,
    pub first_repayment_date: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct RepaymentRequest {
    pub amount: Decimal,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn create_loan(
    State(svc): State<LoanServiceState>,
    Json(req): Json<CreateLoanRequest>,
) -> impl IntoResponse {
    let cmd = CreateLoanApplicationCommand {
        tenant_id: TenantId(req.tenant_id),
        customer_id: CustomerId(req.customer_id),
        account_id: AccountId(req.account_id),
        principal: req.principal,
        interest_rate: req.interest_rate,
        interest_type: req.interest_type,
        term_months: req.term_months,
        repayment_schedule_type: req.repayment_schedule_type,
        first_repayment_date: req.first_repayment_date,
    };
    match svc.create_application(cmd).await {
        Ok(loan) => (StatusCode::CREATED, Json(serde_json::json!(loan))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn approve_loan(
    State(svc): State<LoanServiceState>,
    Path((tenant_id, loan_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let cmd = ApproveLoanCommand {
        loan_id: LoanId(loan_id),
        tenant_id: TenantId(tenant_id),
        approved_by: Uuid::new_v4(), // would come from auth context in production
    };
    match svc.approve_loan(cmd).await {
        Ok(loan) => (StatusCode::OK, Json(serde_json::json!(loan))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn disburse_loan(
    State(svc): State<LoanServiceState>,
    Path((tenant_id, loan_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let cmd = DisburseLoanCommand {
        loan_id: LoanId(loan_id),
        tenant_id: TenantId(tenant_id),
        disbursed_by: Uuid::new_v4(),
    };
    match svc.disburse_loan(cmd).await {
        Ok(loan) => (StatusCode::OK, Json(serde_json::json!(loan))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn repay_loan(
    State(svc): State<LoanServiceState>,
    Path((tenant_id, loan_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<RepaymentRequest>,
) -> impl IntoResponse {
    let cmd = ProcessRepaymentCommand {
        loan_id: LoanId(loan_id),
        tenant_id: TenantId(tenant_id),
        amount: req.amount,
    };
    match svc.process_repayment(cmd).await {
        Ok(loan) => (StatusCode::OK, Json(serde_json::json!(loan))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn get_loan(
    State(svc): State<LoanServiceState>,
    Path((tenant_id, loan_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match svc.get_loan(LoanId(loan_id), TenantId(tenant_id)).await {
        Ok(loan) => (StatusCode::OK, Json(serde_json::json!(loan))).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

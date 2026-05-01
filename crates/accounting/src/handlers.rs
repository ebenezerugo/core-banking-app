use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use domain::accounting::{
    AccountClass, ChartOfAccount, DebitCredit, JournalEntry, JournalLine, JournalStatus,
};
use shared::types::{Currency, JournalEntryId, TenantId};

use crate::repository::PostgresAccountingRepository;
use crate::service::AccountingService;

pub type AccountingServiceState = Arc<AccountingService<PostgresAccountingRepository>>;

#[derive(Debug, Deserialize)]
pub struct PostJournalRequest {
    pub tenant_id: Uuid,
    pub reference: String,
    pub description: String,
    pub transaction_date: NaiveDate,
    pub created_by: Uuid,
    pub lines: Vec<JournalLineRequest>,
}

#[derive(Debug, Deserialize)]
pub struct JournalLineRequest {
    pub account_id: Uuid,
    pub entry_type: DebitCredit,
    pub amount: rust_decimal::Decimal,
    pub currency: Currency,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrialBalanceQuery {
    pub tenant_id: Uuid,
    pub as_of: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn post_journal_entry(
    State(svc): State<AccountingServiceState>,
    Json(req): Json<PostJournalRequest>,
) -> impl IntoResponse {
    let je_id = JournalEntryId::new();
    let lines: Vec<JournalLine> = req
        .lines
        .into_iter()
        .map(|l| JournalLine {
            id: Uuid::new_v4(),
            journal_entry_id: je_id,
            account_id: l.account_id,
            entry_type: l.entry_type,
            amount: l.amount,
            currency: l.currency,
            description: l.description,
        })
        .collect();

    let entry = JournalEntry {
        id: je_id,
        tenant_id: TenantId(req.tenant_id),
        reference: req.reference,
        description: req.description,
        transaction_date: req.transaction_date,
        lines,
        status: JournalStatus::Draft,
        created_by: req.created_by,
        created_at: chrono::Utc::now(),
    };

    match svc.post_journal_entry(entry).await {
        Ok(je) => (StatusCode::CREATED, Json(serde_json::json!(je))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

pub async fn get_trial_balance(
    State(svc): State<AccountingServiceState>,
    Query(q): Query<TrialBalanceQuery>,
) -> impl IntoResponse {
    match svc.get_trial_balance(TenantId(q.tenant_id), q.as_of).await {
        Ok(tb) => {
            let result: Vec<_> = tb
                .into_iter()
                .map(|(coa, bal)| serde_json::json!({ "account": coa, "balance": bal }))
                .collect();
            (StatusCode::OK, Json(serde_json::json!(result))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError { error: e.to_string() })).into_response(),
    }
}

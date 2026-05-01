use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::types::{AccountId, Currency, TransactionId, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Transfer,
    LoanDisbursement,
    LoanRepayment,
    FeeCharge,
    InterestApplication,
    Reversal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Reversed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub tenant_id: TenantId,
    pub idempotency_key: String,
    pub transaction_type: TransactionType,
    pub from_account_id: Option<AccountId>,
    pub to_account_id: Option<AccountId>,
    pub amount: Decimal,
    pub currency: Currency,
    pub fee: Decimal,
    pub description: String,
    pub reference: String,
    pub status: TransactionStatus,
    pub reversal_of: Option<TransactionId>,
    pub correlation_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionEvent {
    TransactionInitiated { transaction: Transaction },
    TransactionCompleted { transaction_id: TransactionId, completed_at: DateTime<Utc> },
    TransactionFailed { transaction_id: TransactionId, reason: String },
    TransactionReversed { transaction_id: TransactionId, reversal_id: TransactionId },
}

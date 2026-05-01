use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::types::{AccountId, Currency, LoanId, TenantId, TransactionId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChargeType {
    Flat,
    Percentage,
    Tiered,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChargeAppliedTo {
    LoanOrigination,
    LoanRepayment,
    AccountMaintenance,
    Transfer,
    ATMWithdrawal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargeDefinition {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub charge_type: ChargeType,
    pub applied_to: ChargeAppliedTo,
    /// Used when charge_type is Flat.
    pub amount: Decimal,
    /// Used when charge_type is Percentage (0–100).
    pub percentage: Decimal,
    pub currency: Currency,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedCharge {
    pub id: Uuid,
    pub charge_definition_id: Uuid,
    pub account_id: Option<AccountId>,
    pub loan_id: Option<LoanId>,
    pub transaction_id: Option<TransactionId>,
    pub amount: Decimal,
    pub currency: Currency,
    pub waived: bool,
    pub waiver_reason: Option<String>,
    pub applied_at: DateTime<Utc>,
}

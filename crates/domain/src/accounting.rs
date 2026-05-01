use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::types::{Currency, JournalEntryId, TenantId};
use uuid::Uuid;
use shared::errors::DomainError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountClass {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DebitCredit {
    Debit,
    Credit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JournalStatus {
    Draft,
    Posted,
    Reversed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartOfAccount {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub code: String,
    pub name: String,
    pub account_class: AccountClass,
    pub normal_balance: DebitCredit,
    pub parent_id: Option<Uuid>,
    pub is_leaf: bool,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalLine {
    pub id: Uuid,
    pub journal_entry_id: JournalEntryId,
    pub account_id: Uuid,
    pub entry_type: DebitCredit,
    pub amount: Decimal,
    pub currency: Currency,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    pub tenant_id: TenantId,
    pub reference: String,
    pub description: String,
    pub transaction_date: NaiveDate,
    pub lines: Vec<JournalLine>,
    pub status: JournalStatus,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<Utc>,
}

impl JournalEntry {
    /// Enforces the double-entry invariant: Σ debits == Σ credits.
    pub fn validate(&self) -> Result<(), DomainError> {
        let total_debits: Decimal = self
            .lines
            .iter()
            .filter(|l| l.entry_type == DebitCredit::Debit)
            .map(|l| l.amount)
            .sum();
        let total_credits: Decimal = self
            .lines
            .iter()
            .filter(|l| l.entry_type == DebitCredit::Credit)
            .map(|l| l.amount)
            .sum();
        if total_debits != total_credits {
            return Err(DomainError::ValidationError(format!(
                "Journal entry not balanced: debits={}, credits={}",
                total_debits, total_credits
            )));
        }
        Ok(())
    }
}

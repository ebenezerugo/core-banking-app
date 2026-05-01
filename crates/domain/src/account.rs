use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::errors::DomainError;
use shared::types::{AccountId, Currency, CustomerId, TenantId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Savings,
    Current,
    FixedDeposit,
    RecurringDeposit,
    Loan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Dormant,
    Frozen,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub tenant_id: TenantId,
    pub account_number: String,
    pub customer_id: CustomerId,
    pub account_type: AccountType,
    pub currency: Currency,
    pub balance: Decimal,
    pub available_balance: Decimal,
    pub status: AccountStatus,
    pub interest_rate: Option<Decimal>,
    pub maturity_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Used for optimistic locking.
    pub version: i64,
}

impl Account {
    pub fn credit(&mut self, amount: Decimal) -> Result<(), DomainError> {
        if amount <= Decimal::ZERO {
            return Err(DomainError::NegativeAmount);
        }
        self.balance += amount;
        self.available_balance += amount;
        self.version += 1;
        Ok(())
    }

    pub fn debit(&mut self, amount: Decimal) -> Result<(), DomainError> {
        if amount <= Decimal::ZERO {
            return Err(DomainError::NegativeAmount);
        }
        if self.available_balance < amount {
            return Err(DomainError::InsufficientFunds {
                available: self.available_balance.to_string(),
                required: amount.to_string(),
            });
        }
        self.balance -= amount;
        self.available_balance -= amount;
        self.version += 1;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountEvent {
    AccountOpened { account: Account },
    AccountCredited { account_id: AccountId, amount: Decimal, currency: Currency, reference: String },
    AccountDebited { account_id: AccountId, amount: Decimal, currency: Currency, reference: String },
    AccountFrozen { account_id: AccountId, reason: String },
    AccountClosed { account_id: AccountId, reason: String },
    InterestApplied { account_id: AccountId, amount: Decimal, period: String },
}

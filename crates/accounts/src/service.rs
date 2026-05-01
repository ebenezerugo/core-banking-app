use std::sync::Arc;

use chrono::Utc;
use domain::account::{Account, AccountEvent, AccountStatus, AccountType};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::events::{EventEnvelope, EventStore};
use shared::pagination::PageRequest;
use shared::types::{AccountId, Currency, CustomerId, TenantId};
use tracing::instrument;
use uuid::Uuid;

use crate::repository::AccountRepository;

pub struct AccountService<R: AccountRepository> {
    repository: R,
    event_store: Arc<dyn EventStore>,
}

impl<R: AccountRepository> AccountService<R> {
    pub fn new(repository: R, event_store: Arc<dyn EventStore>) -> Self {
        Self { repository, event_store }
    }

    #[instrument(skip(self))]
    pub async fn open_account(
        &self,
        tenant_id: TenantId,
        customer_id: CustomerId,
        account_type: AccountType,
        currency: Currency,
        interest_rate: Option<Decimal>,
    ) -> Result<Account, DomainError> {
        let account = Account {
            id: AccountId::new(),
            tenant_id,
            account_number: generate_account_number(),
            customer_id,
            account_type,
            currency,
            balance: Decimal::ZERO,
            available_balance: Decimal::ZERO,
            status: AccountStatus::Active,
            interest_rate,
            maturity_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 0,
        };

        self.repository.create(&account).await?;

        let event = AccountEvent::AccountOpened { account: account.clone() };
        let _ = self.event_store.append(EventEnvelope::new(
            account.id.0,
            "Account",
            "AccountOpened",
            serde_json::to_value(&event).unwrap_or_default(),
        )).await;

        Ok(account)
    }

    #[instrument(skip(self))]
    pub async fn deposit(
        &self,
        account_id: AccountId,
        tenant_id: TenantId,
        amount: Decimal,
        reference: String,
    ) -> Result<Account, DomainError> {
        let mut account = self
            .repository
            .find_by_id(account_id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound(account_id.to_string()))?;

        if account.status != AccountStatus::Active {
            return Err(DomainError::InvalidStateTransition(
                "Account is not active".into(),
            ));
        }

        account.credit(amount)?;
        account.updated_at = Utc::now();
        self.repository.update(&account).await?;

        let event = AccountEvent::AccountCredited {
            account_id,
            amount,
            currency: account.currency,
            reference: reference.clone(),
        };
        let _ = self.event_store.append(EventEnvelope::new(
            account.id.0, "Account", "AccountCredited",
            serde_json::to_value(&event).unwrap_or_default(),
        )).await;

        Ok(account)
    }

    #[instrument(skip(self))]
    pub async fn withdraw(
        &self,
        account_id: AccountId,
        tenant_id: TenantId,
        amount: Decimal,
        reference: String,
    ) -> Result<Account, DomainError> {
        let mut account = self
            .repository
            .find_by_id(account_id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound(account_id.to_string()))?;

        if account.status != AccountStatus::Active {
            return Err(DomainError::InvalidStateTransition(
                "Account is not active".into(),
            ));
        }

        account.debit(amount)?;
        account.updated_at = Utc::now();
        self.repository.update(&account).await?;

        let event = AccountEvent::AccountDebited {
            account_id,
            amount,
            currency: account.currency,
            reference,
        };
        let _ = self.event_store.append(EventEnvelope::new(
            account.id.0, "Account", "AccountDebited",
            serde_json::to_value(&event).unwrap_or_default(),
        )).await;

        Ok(account)
    }

    pub async fn get_account(
        &self,
        id: AccountId,
        tenant_id: TenantId,
    ) -> Result<Account, DomainError> {
        self.repository
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::AccountNotFound(id.to_string()))
    }

    pub async fn list_by_customer(
        &self,
        customer_id: CustomerId,
        tenant_id: TenantId,
    ) -> Result<Vec<Account>, DomainError> {
        self.repository.list_by_customer(customer_id, tenant_id).await
    }
}

fn generate_account_number() -> String {
    // Simple sequential-looking account number using UUID entropy
    let id = Uuid::new_v4();
    let bytes = id.as_bytes();
    let num: u64 = u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
    ]);
    format!("{:016}", num % 10_000_000_000_000_000u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_credit_increases_balance() {
        let mut account = Account {
            id: AccountId::new(),
            tenant_id: TenantId::new(),
            account_number: "0000000000000001".into(),
            customer_id: CustomerId::new(),
            account_type: AccountType::Savings,
            currency: Currency::USD,
            balance: Decimal::from(100),
            available_balance: Decimal::from(100),
            status: AccountStatus::Active,
            interest_rate: None,
            maturity_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 0,
        };
        account.credit(Decimal::from(50)).unwrap();
        assert_eq!(account.balance, Decimal::from(150));
        assert_eq!(account.available_balance, Decimal::from(150));
        assert_eq!(account.version, 1);
    }

    #[test]
    fn test_account_debit_insufficient_funds() {
        let mut account = Account {
            id: AccountId::new(),
            tenant_id: TenantId::new(),
            account_number: "0000000000000002".into(),
            customer_id: CustomerId::new(),
            account_type: AccountType::Savings,
            currency: Currency::USD,
            balance: Decimal::from(100),
            available_balance: Decimal::from(100),
            status: AccountStatus::Active,
            interest_rate: None,
            maturity_date: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 0,
        };
        let result = account.debit(Decimal::from(200));
        assert!(matches!(result, Err(DomainError::InsufficientFunds { .. })));
    }
}

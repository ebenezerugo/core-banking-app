use std::sync::Arc;

use chrono::{NaiveDate, Utc};
use domain::accounting::{JournalEntry, JournalStatus};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::types::{JournalEntryId, TenantId};
use tracing::instrument;
use uuid::Uuid;

use crate::repository::AccountingRepository;

pub struct AccountingService<R: AccountingRepository> {
    repository: R,
}

impl<R: AccountingRepository> AccountingService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Validate and post a journal entry to the general ledger.
    #[instrument(skip(self, entry))]
    pub async fn post_journal_entry(
        &self,
        mut entry: JournalEntry,
    ) -> Result<JournalEntry, DomainError> {
        // Enforce double-entry invariant before persisting
        entry.validate()?;

        entry.status = JournalStatus::Posted;
        self.repository.post_journal_entry(&entry).await?;
        Ok(entry)
    }

    pub async fn get_account_balance(
        &self,
        account_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Decimal, DomainError> {
        self.repository.get_account_balance(account_id, tenant_id).await
    }

    pub async fn get_trial_balance(
        &self,
        tenant_id: TenantId,
        as_of: NaiveDate,
    ) -> Result<Vec<(domain::accounting::ChartOfAccount, Decimal)>, DomainError> {
        self.repository.get_trial_balance(tenant_id, as_of).await
    }

    pub async fn get_journal_entry(
        &self,
        id: JournalEntryId,
        tenant_id: TenantId,
    ) -> Result<JournalEntry, DomainError> {
        self.repository
            .get_journal_entry(id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::accounting::{DebitCredit, JournalLine, JournalStatus};
    use rust_decimal::Decimal;
    use shared::types::{Currency, JournalEntryId, TenantId};
    use uuid::Uuid;

    fn make_entry(lines: Vec<JournalLine>) -> JournalEntry {
        JournalEntry {
            id: JournalEntryId::new(),
            tenant_id: TenantId::new(),
            reference: "TEST-001".into(),
            description: "Test entry".into(),
            transaction_date: chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            lines,
            status: JournalStatus::Draft,
            created_by: Uuid::new_v4(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_balanced_journal_entry_validates() {
        let account_id = Uuid::new_v4();
        let je_id = JournalEntryId::new();
        let lines = vec![
            JournalLine {
                id: Uuid::new_v4(),
                journal_entry_id: je_id,
                account_id,
                entry_type: DebitCredit::Debit,
                amount: Decimal::from(100),
                currency: Currency::USD,
                description: None,
            },
            JournalLine {
                id: Uuid::new_v4(),
                journal_entry_id: je_id,
                account_id,
                entry_type: DebitCredit::Credit,
                amount: Decimal::from(100),
                currency: Currency::USD,
                description: None,
            },
        ];
        let entry = make_entry(lines);
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn test_unbalanced_journal_entry_fails() {
        let account_id = Uuid::new_v4();
        let je_id = JournalEntryId::new();
        let lines = vec![
            JournalLine {
                id: Uuid::new_v4(),
                journal_entry_id: je_id,
                account_id,
                entry_type: DebitCredit::Debit,
                amount: Decimal::from(100),
                currency: Currency::USD,
                description: None,
            },
            JournalLine {
                id: Uuid::new_v4(),
                journal_entry_id: je_id,
                account_id,
                entry_type: DebitCredit::Credit,
                amount: Decimal::from(90),
                currency: Currency::USD,
                description: None,
            },
        ];
        let entry = make_entry(lines);
        assert!(matches!(
            entry.validate(),
            Err(DomainError::ValidationError(_))
        ));
    }
}

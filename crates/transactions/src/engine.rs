use std::sync::Arc;

use chrono::Utc;
use rust_decimal::Decimal;
use domain::transaction::{Transaction, TransactionStatus, TransactionType};
use shared::errors::DomainError;
use shared::events::{EventEnvelope, EventStore};
use shared::types::{AccountId, Currency, TenantId, TransactionId};
use tracing::instrument;
use uuid::Uuid;

use crate::repository::{PostgresTransactionRepository, TransactionRepository, ToDbStr};

pub struct ExecuteTransactionCommand {
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
}

pub struct TransactionEngine {
    pool: Arc<sqlx::PgPool>,
    repository: Arc<PostgresTransactionRepository>,
    event_store: Arc<dyn EventStore>,
}

impl TransactionEngine {
    pub fn new(
        pool: Arc<sqlx::PgPool>,
        repository: Arc<PostgresTransactionRepository>,
        event_store: Arc<dyn EventStore>,
    ) -> Self {
        Self { pool, repository, event_store }
    }

    #[instrument(skip(self, cmd))]
    pub async fn execute(&self, cmd: ExecuteTransactionCommand) -> Result<Transaction, DomainError> {
        // Idempotency check
        if let Some(existing) = self.repository.find_by_idempotency_key(&cmd.idempotency_key, cmd.tenant_id).await? {
            return Ok(existing);
        }

        let transaction_id = TransactionId::new();
        let mut tx = self.pool.begin().await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        // Debit source account
        if let Some(from_id) = cmd.from_account_id {
            let rows_affected = sqlx::query(
                r#"UPDATE accounts SET balance = balance - $1, available_balance = available_balance - $1, version = version + 1
                   WHERE id = $2 AND available_balance >= $1"#,
            )
            .bind(cmd.amount).bind(from_id.0)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::ValidationError(e.to_string()))?
            .rows_affected();

            if rows_affected == 0 {
                return Err(DomainError::InsufficientFunds { available: "unknown".into(), required: cmd.amount.to_string() });
            }
        }

        // Credit destination account
        if let Some(to_id) = cmd.to_account_id {
            sqlx::query(
                r#"UPDATE accounts SET balance = balance + $1, available_balance = available_balance + $1, version = version + 1
                   WHERE id = $2"#,
            )
            .bind(cmd.amount).bind(to_id.0)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;
        }

        let now = Utc::now();
        let transaction = Transaction {
            id: transaction_id,
            tenant_id: cmd.tenant_id,
            idempotency_key: cmd.idempotency_key,
            transaction_type: cmd.transaction_type,
            from_account_id: cmd.from_account_id,
            to_account_id: cmd.to_account_id,
            amount: cmd.amount,
            currency: cmd.currency,
            fee: cmd.fee,
            description: cmd.description,
            reference: cmd.reference,
            status: TransactionStatus::Completed,
            reversal_of: None,
            correlation_id: Uuid::new_v4(),
            created_at: now,
            completed_at: Some(now),
        };

        sqlx::query(
            r#"INSERT INTO transactions (id, tenant_id, idempotency_key, transaction_type, from_account_id, to_account_id,
               amount, currency, fee, description, reference, status, correlation_id, created_at, completed_at)
               VALUES ($1,$2,$3,$4::transaction_type,$5,$6,$7,$8,$9,$10,$11,$12::transaction_status,$13,$14,$15)"#,
        )
        .bind(transaction.id.0).bind(transaction.tenant_id.0).bind(&transaction.idempotency_key)
        .bind(transaction.transaction_type.to_db_str())
        .bind(transaction.from_account_id.map(|a| a.0)).bind(transaction.to_account_id.map(|a| a.0))
        .bind(transaction.amount).bind(transaction.currency.to_string()).bind(transaction.fee)
        .bind(&transaction.description).bind(&transaction.reference)
        .bind(transaction.status.to_db_str())
        .bind(transaction.correlation_id).bind(transaction.created_at).bind(transaction.completed_at)
        .execute(&mut *tx).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        tx.commit().await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        use domain::transaction::TransactionEvent;
        let event = TransactionEvent::TransactionCompleted { transaction_id: transaction.id, completed_at: now };
        let _ = self.event_store.append(EventEnvelope::new(
            transaction.id.0, "Transaction", "TransactionCompleted",
            serde_json::to_value(&event).unwrap_or_default(),
        )).await;

        Ok(transaction)
    }

    #[instrument(skip(self))]
    pub async fn reverse(&self, transaction_id: TransactionId, tenant_id: TenantId, reason: String) -> Result<Transaction, DomainError> {
        let original = self.repository.find_by_id(transaction_id, tenant_id).await?
            .ok_or_else(|| DomainError::TransactionNotFound(transaction_id.to_string()))?;

        if original.status != TransactionStatus::Completed {
            return Err(DomainError::InvalidStateTransition(format!("Cannot reverse transaction in state {:?}", original.status)));
        }

        let reversal = self.execute(ExecuteTransactionCommand {
            tenant_id,
            idempotency_key: format!("reversal-{}", transaction_id),
            transaction_type: TransactionType::Reversal,
            from_account_id: original.to_account_id,
            to_account_id: original.from_account_id,
            amount: original.amount,
            currency: original.currency,
            fee: Decimal::ZERO,
            description: format!("Reversal: {}", reason),
            reference: format!("REV-{}", original.reference),
        }).await?;

        self.repository.update_status(transaction_id, TransactionStatus::Reversed, None).await?;
        Ok(reversal)
    }
}

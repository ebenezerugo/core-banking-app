use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::transaction::{Transaction, TransactionStatus, TransactionType};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::pagination::{PageRequest, PageResponse};
use shared::types::{AccountId, Currency, TenantId, TransactionId};
use sqlx::{PgPool, Row};
use tracing::instrument;
use uuid::Uuid;

#[async_trait]
pub trait TransactionRepository: Send + Sync {
    async fn create(&self, tx: &Transaction) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: TransactionId, tenant_id: TenantId) -> Result<Option<Transaction>, DomainError>;
    async fn find_by_idempotency_key(&self, key: &str, tenant_id: TenantId) -> Result<Option<Transaction>, DomainError>;
    async fn update_status(&self, id: TransactionId, status: TransactionStatus, completed_at: Option<DateTime<Utc>>) -> Result<(), DomainError>;
    async fn list_by_account(&self, account_id: AccountId, tenant_id: TenantId, page: PageRequest) -> Result<PageResponse<Transaction>, DomainError>;
}

pub struct PostgresTransactionRepository { pub(crate) pool: PgPool }
impl PostgresTransactionRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

fn parse_currency(s: &str) -> Currency {
    match s { "EUR" => Currency::EUR, "GBP" => Currency::GBP, "KES" => Currency::KES,
              "NGN" => Currency::NGN, "GHS" => Currency::GHS, "ZAR" => Currency::ZAR, _ => Currency::USD }
}

fn row_to_tx(r: &sqlx::postgres::PgRow) -> Transaction {
    Transaction {
        id: TransactionId(r.get::<Uuid, _>("id")),
        tenant_id: TenantId(r.get::<Uuid, _>("tenant_id")),
        idempotency_key: r.get("idempotency_key"),
        transaction_type: match r.get::<String, _>("transaction_type").as_str() {
            "withdrawal" => TransactionType::Withdrawal, "transfer" => TransactionType::Transfer,
            "loan_disbursement" => TransactionType::LoanDisbursement, "loan_repayment" => TransactionType::LoanRepayment,
            "fee_charge" => TransactionType::FeeCharge, "interest_application" => TransactionType::InterestApplication,
            "reversal" => TransactionType::Reversal, _ => TransactionType::Deposit,
        },
        from_account_id: r.get::<Option<Uuid>, _>("from_account_id").map(AccountId),
        to_account_id: r.get::<Option<Uuid>, _>("to_account_id").map(AccountId),
        amount: r.get("amount"),
        currency: parse_currency(&r.get::<String, _>("currency")),
        fee: r.get("fee"),
        description: r.get("description"),
        reference: r.get("reference"),
        status: match r.get::<String, _>("status").as_str() {
            "processing" => TransactionStatus::Processing, "completed" => TransactionStatus::Completed,
            "failed" => TransactionStatus::Failed, "reversed" => TransactionStatus::Reversed, _ => TransactionStatus::Pending,
        },
        reversal_of: r.get::<Option<Uuid>, _>("reversal_of").map(TransactionId),
        correlation_id: r.get("correlation_id"),
        created_at: r.get("created_at"),
        completed_at: r.get("completed_at"),
    }
}

pub trait ToDbStr { fn to_db_str(&self) -> &'static str; }
impl ToDbStr for TransactionType {
    fn to_db_str(&self) -> &'static str {
        match self { TransactionType::Deposit => "deposit", TransactionType::Withdrawal => "withdrawal",
            TransactionType::Transfer => "transfer", TransactionType::LoanDisbursement => "loan_disbursement",
            TransactionType::LoanRepayment => "loan_repayment", TransactionType::FeeCharge => "fee_charge",
            TransactionType::InterestApplication => "interest_application", TransactionType::Reversal => "reversal" }
    }
}
impl ToDbStr for TransactionStatus {
    fn to_db_str(&self) -> &'static str {
        match self { TransactionStatus::Pending => "pending", TransactionStatus::Processing => "processing",
            TransactionStatus::Completed => "completed", TransactionStatus::Failed => "failed",
            TransactionStatus::Reversed => "reversed" }
    }
}

#[async_trait]
impl TransactionRepository for PostgresTransactionRepository {
    async fn create(&self, tx: &Transaction) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO transactions (id, tenant_id, idempotency_key, transaction_type, from_account_id, to_account_id,
               amount, currency, fee, description, reference, status, reversal_of, correlation_id, created_at, completed_at)
               VALUES ($1,$2,$3,$4::transaction_type,$5,$6,$7,$8,$9,$10,$11,$12::transaction_status,$13,$14,$15,$16)"#,
        )
        .bind(tx.id.0).bind(tx.tenant_id.0).bind(&tx.idempotency_key).bind(tx.transaction_type.to_db_str())
        .bind(tx.from_account_id.map(|a| a.0)).bind(tx.to_account_id.map(|a| a.0))
        .bind(tx.amount).bind(tx.currency.to_string()).bind(tx.fee)
        .bind(&tx.description).bind(&tx.reference).bind(tx.status.to_db_str())
        .bind(tx.reversal_of.map(|r| r.0)).bind(tx.correlation_id).bind(tx.created_at).bind(tx.completed_at)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn find_by_id(&self, id: TransactionId, tenant_id: TenantId) -> Result<Option<Transaction>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, idempotency_key, transaction_type::text, from_account_id, to_account_id,
                      amount, currency, fee, description, reference, status::text, reversal_of, correlation_id, created_at, completed_at
               FROM transactions WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id.0).bind(tenant_id.0)
        .fetch_optional(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_tx))
    }

    async fn find_by_idempotency_key(&self, key: &str, tenant_id: TenantId) -> Result<Option<Transaction>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, idempotency_key, transaction_type::text, from_account_id, to_account_id,
                      amount, currency, fee, description, reference, status::text, reversal_of, correlation_id, created_at, completed_at
               FROM transactions WHERE idempotency_key = $1 AND tenant_id = $2"#,
        )
        .bind(key).bind(tenant_id.0)
        .fetch_optional(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_tx))
    }

    async fn update_status(&self, id: TransactionId, status: TransactionStatus, completed_at: Option<DateTime<Utc>>) -> Result<(), DomainError> {
        sqlx::query("UPDATE transactions SET status = $2::transaction_status, completed_at = $3 WHERE id = $1")
        .bind(id.0).bind(status.to_db_str()).bind(completed_at)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn list_by_account(&self, account_id: AccountId, tenant_id: TenantId, page: PageRequest) -> Result<PageResponse<Transaction>, DomainError> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transactions WHERE tenant_id = $1 AND (from_account_id = $2 OR to_account_id = $2)"
        )
        .bind(tenant_id.0).bind(account_id.0)
        .fetch_one(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let rows = sqlx::query(
            r#"SELECT id, tenant_id, idempotency_key, transaction_type::text, from_account_id, to_account_id,
                      amount, currency, fee, description, reference, status::text, reversal_of, correlation_id, created_at, completed_at
               FROM transactions
               WHERE tenant_id = $1 AND (from_account_id = $2 OR to_account_id = $2)
               ORDER BY created_at DESC LIMIT $3 OFFSET $4"#,
        )
        .bind(tenant_id.0).bind(account_id.0).bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let items = rows.iter().map(row_to_tx).collect();
        Ok(PageResponse::new(items, total, &page))
    }
}

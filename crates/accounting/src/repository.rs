use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use domain::accounting::{AccountClass, ChartOfAccount, DebitCredit, JournalEntry, JournalLine, JournalStatus};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::types::{Currency, JournalEntryId, TenantId};
use sqlx::{PgPool, Row};
use tracing::instrument;
use uuid::Uuid;

#[async_trait]
pub trait AccountingRepository: Send + Sync {
    async fn create_chart_account(&self, coa: &ChartOfAccount) -> Result<(), DomainError>;
    async fn post_journal_entry(&self, entry: &JournalEntry) -> Result<(), DomainError>;
    async fn get_journal_entry(&self, id: JournalEntryId, tenant_id: TenantId) -> Result<Option<JournalEntry>, DomainError>;
    async fn get_account_balance(&self, account_id: Uuid, tenant_id: TenantId) -> Result<Decimal, DomainError>;
    async fn get_trial_balance(&self, tenant_id: TenantId, as_of: NaiveDate) -> Result<Vec<(ChartOfAccount, Decimal)>, DomainError>;
}

pub struct PostgresAccountingRepository { pool: PgPool }
impl PostgresAccountingRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

fn parse_currency(s: &str) -> Currency {
    match s { "EUR" => Currency::EUR, "GBP" => Currency::GBP, "KES" => Currency::KES,
              "NGN" => Currency::NGN, "GHS" => Currency::GHS, "ZAR" => Currency::ZAR,
              "UGX" => Currency::UGX, "TZS" => Currency::TZS, "RWF" => Currency::RWF, _ => Currency::USD }
}

trait ToDbStr { fn to_db_str(&self) -> &'static str; }
impl ToDbStr for AccountClass {
    fn to_db_str(&self) -> &'static str {
        match self { AccountClass::Asset => "asset", AccountClass::Liability => "liability",
            AccountClass::Equity => "equity", AccountClass::Income => "income", AccountClass::Expense => "expense" }
    }
}
impl ToDbStr for DebitCredit {
    fn to_db_str(&self) -> &'static str { match self { DebitCredit::Debit => "debit", DebitCredit::Credit => "credit" } }
}
impl ToDbStr for JournalStatus {
    fn to_db_str(&self) -> &'static str {
        match self { JournalStatus::Draft => "draft", JournalStatus::Posted => "posted", JournalStatus::Reversed => "reversed" }
    }
}

#[async_trait]
impl AccountingRepository for PostgresAccountingRepository {
    async fn create_chart_account(&self, coa: &ChartOfAccount) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO chart_of_accounts (id, tenant_id, code, name, account_class, normal_balance, parent_id, is_leaf, currency)
               VALUES ($1,$2,$3,$4,$5::account_class,$6::debit_credit,$7,$8,$9)"#,
        )
        .bind(coa.id).bind(coa.tenant_id.0).bind(&coa.code).bind(&coa.name)
        .bind(coa.account_class.to_db_str()).bind(coa.normal_balance.to_db_str())
        .bind(coa.parent_id).bind(coa.is_leaf).bind(coa.currency.to_string())
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn post_journal_entry(&self, entry: &JournalEntry) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        sqlx::query(
            r#"INSERT INTO journal_entries (id, tenant_id, reference, description, transaction_date, status, created_by, created_at)
               VALUES ($1,$2,$3,$4,$5,$6::journal_status,$7,$8)"#,
        )
        .bind(entry.id.0).bind(entry.tenant_id.0).bind(&entry.reference).bind(&entry.description)
        .bind(entry.transaction_date).bind(entry.status.to_db_str()).bind(entry.created_by).bind(entry.created_at)
        .execute(&mut *tx).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        for line in &entry.lines {
            sqlx::query(
                r#"INSERT INTO journal_lines (id, journal_entry_id, account_id, entry_type, amount, currency, description)
                   VALUES ($1,$2,$3,$4::debit_credit,$5,$6,$7)"#,
            )
            .bind(line.id).bind(line.journal_entry_id.0).bind(line.account_id)
            .bind(line.entry_type.to_db_str()).bind(line.amount).bind(line.currency.to_string()).bind(&line.description)
            .execute(&mut *tx).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn get_journal_entry(&self, id: JournalEntryId, tenant_id: TenantId) -> Result<Option<JournalEntry>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, reference, description, transaction_date, status::text, created_by, created_at
               FROM journal_entries WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id.0).bind(tenant_id.0)
        .fetch_optional(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let Some(r) = row else { return Ok(None); };

        let lines = sqlx::query(
            "SELECT id, journal_entry_id, account_id, entry_type::text, amount, currency, description FROM journal_lines WHERE journal_entry_id = $1",
        )
        .bind(id.0).fetch_all(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let lines: Vec<JournalLine> = lines.iter().map(|l| JournalLine {
            id: l.get("id"),
            journal_entry_id: JournalEntryId(l.get::<Uuid, _>("journal_entry_id")),
            account_id: l.get("account_id"),
            entry_type: match l.get::<String, _>("entry_type").as_str() { "credit" => DebitCredit::Credit, _ => DebitCredit::Debit },
            amount: l.get("amount"),
            currency: parse_currency(&l.get::<String, _>("currency")),
            description: l.get("description"),
        }).collect();

        Ok(Some(JournalEntry {
            id: JournalEntryId(r.get::<Uuid, _>("id")),
            tenant_id: TenantId(r.get::<Uuid, _>("tenant_id")),
            reference: r.get("reference"),
            description: r.get("description"),
            transaction_date: r.get("transaction_date"),
            lines,
            status: match r.get::<String, _>("status").as_str() {
                "posted" => JournalStatus::Posted, "reversed" => JournalStatus::Reversed, _ => JournalStatus::Draft,
            },
            created_by: r.get("created_by"),
            created_at: r.get("created_at"),
        }))
    }

    async fn get_account_balance(&self, account_id: Uuid, _tenant_id: TenantId) -> Result<Decimal, DomainError> {
        let debits: Decimal = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(jl.amount), 0) FROM journal_lines jl
               JOIN journal_entries je ON je.id = jl.journal_entry_id
               WHERE jl.account_id = $1 AND jl.entry_type = 'debit' AND je.status = 'posted'"#,
        )
        .bind(account_id).fetch_one(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let credits: Decimal = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(jl.amount), 0) FROM journal_lines jl
               JOIN journal_entries je ON je.id = jl.journal_entry_id
               WHERE jl.account_id = $1 AND jl.entry_type = 'credit' AND je.status = 'posted'"#,
        )
        .bind(account_id).fetch_one(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        Ok(debits - credits)
    }

    async fn get_trial_balance(&self, tenant_id: TenantId, _as_of: NaiveDate) -> Result<Vec<(ChartOfAccount, Decimal)>, DomainError> {
        let rows = sqlx::query(
            r#"SELECT id, tenant_id, code, name, account_class::text, normal_balance::text, parent_id, is_leaf, currency
               FROM chart_of_accounts WHERE tenant_id = $1 AND is_leaf = true"#,
        )
        .bind(tenant_id.0).fetch_all(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let mut result = Vec::new();
        for row in &rows {
            let coa = ChartOfAccount {
                id: row.get("id"),
                tenant_id: TenantId(row.get::<Uuid, _>("tenant_id")),
                code: row.get("code"),
                name: row.get("name"),
                account_class: match row.get::<String, _>("account_class").as_str() {
                    "liability" => AccountClass::Liability, "equity" => AccountClass::Equity,
                    "income" => AccountClass::Income, "expense" => AccountClass::Expense, _ => AccountClass::Asset,
                },
                normal_balance: match row.get::<String, _>("normal_balance").as_str() { "credit" => DebitCredit::Credit, _ => DebitCredit::Debit },
                parent_id: row.get("parent_id"),
                is_leaf: row.get("is_leaf"),
                currency: parse_currency(&row.get::<String, _>("currency")),
            };
            let balance = self.get_account_balance(coa.id, tenant_id).await?;
            result.push((coa, balance));
        }
        Ok(result)
    }
}

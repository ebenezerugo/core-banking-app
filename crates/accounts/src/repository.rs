use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::account::{Account, AccountStatus, AccountType};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::pagination::{PageRequest, PageResponse};
use shared::types::{AccountId, Currency, CustomerId, TenantId};
use sqlx::{PgPool, Row};
use tracing::instrument;
use uuid::Uuid;

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn create(&self, account: &Account) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: AccountId, tenant_id: TenantId) -> Result<Option<Account>, DomainError>;
    async fn find_by_number(&self, number: &str, tenant_id: TenantId) -> Result<Option<Account>, DomainError>;
    async fn update(&self, account: &Account) -> Result<(), DomainError>;
    async fn list_by_customer(&self, customer_id: CustomerId, tenant_id: TenantId) -> Result<Vec<Account>, DomainError>;
    async fn list(&self, tenant_id: TenantId, page: PageRequest) -> Result<PageResponse<Account>, DomainError>;
}

pub struct PostgresAccountRepository { pool: PgPool }
impl PostgresAccountRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

fn parse_currency(s: &str) -> Currency {
    match s { "EUR" => Currency::EUR, "GBP" => Currency::GBP, "KES" => Currency::KES,
              "NGN" => Currency::NGN, "GHS" => Currency::GHS, "ZAR" => Currency::ZAR,
              "UGX" => Currency::UGX, "TZS" => Currency::TZS, "RWF" => Currency::RWF, _ => Currency::USD }
}

fn row_to_account(row: &sqlx::postgres::PgRow) -> Account {
    Account {
        id: AccountId(row.get::<Uuid, _>("id")),
        tenant_id: TenantId(row.get::<Uuid, _>("tenant_id")),
        account_number: row.get("account_number"),
        customer_id: CustomerId(row.get::<Uuid, _>("customer_id")),
        account_type: match row.get::<String, _>("account_type").as_str() {
            "current" => AccountType::Current, "fixed_deposit" => AccountType::FixedDeposit,
            "recurring_deposit" => AccountType::RecurringDeposit, "loan" => AccountType::Loan,
            _ => AccountType::Savings,
        },
        currency: parse_currency(&row.get::<String, _>("currency")),
        balance: row.get("balance"),
        available_balance: row.get("available_balance"),
        status: match row.get::<String, _>("status").as_str() {
            "dormant" => AccountStatus::Dormant, "frozen" => AccountStatus::Frozen,
            "closed" => AccountStatus::Closed, _ => AccountStatus::Active,
        },
        interest_rate: row.get("interest_rate"),
        maturity_date: row.get("maturity_date"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        version: row.get("version"),
    }
}

trait ToDbStr { fn to_db_str(&self) -> &'static str; }
impl ToDbStr for AccountType {
    fn to_db_str(&self) -> &'static str {
        match self { AccountType::Savings => "savings", AccountType::Current => "current",
            AccountType::FixedDeposit => "fixed_deposit", AccountType::RecurringDeposit => "recurring_deposit",
            AccountType::Loan => "loan" }
    }
}
impl ToDbStr for AccountStatus {
    fn to_db_str(&self) -> &'static str {
        match self { AccountStatus::Active => "active", AccountStatus::Dormant => "dormant",
            AccountStatus::Frozen => "frozen", AccountStatus::Closed => "closed" }
    }
}

#[async_trait]
impl AccountRepository for PostgresAccountRepository {
    #[instrument(skip(self, account))]
    async fn create(&self, account: &Account) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO accounts (id, tenant_id, account_number, customer_id, account_type, currency,
               balance, available_balance, status, interest_rate, maturity_date, created_at, updated_at, version)
               VALUES ($1,$2,$3,$4,$5::account_type,$6,$7,$8,$9::account_status,$10,$11,$12,$13,$14)"#,
        )
        .bind(account.id.0).bind(account.tenant_id.0).bind(&account.account_number)
        .bind(account.customer_id.0).bind(account.account_type.to_db_str())
        .bind(account.currency.to_string())
        .bind(account.balance).bind(account.available_balance)
        .bind(account.status.to_db_str()).bind(account.interest_rate).bind(account.maturity_date)
        .bind(account.created_at).bind(account.updated_at).bind(account.version)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn find_by_id(&self, id: AccountId, tenant_id: TenantId) -> Result<Option<Account>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, account_number, customer_id, account_type::text, currency,
                      balance, available_balance, status::text, interest_rate, maturity_date,
                      created_at, updated_at, version
               FROM accounts WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id.0).bind(tenant_id.0)
        .fetch_optional(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_account))
    }

    async fn find_by_number(&self, number: &str, tenant_id: TenantId) -> Result<Option<Account>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, account_number, customer_id, account_type::text, currency,
                      balance, available_balance, status::text, interest_rate, maturity_date,
                      created_at, updated_at, version
               FROM accounts WHERE account_number = $1 AND tenant_id = $2"#,
        )
        .bind(number).bind(tenant_id.0)
        .fetch_optional(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_account))
    }

    async fn update(&self, account: &Account) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE accounts SET balance=$3, available_balance=$4, status=$5::account_status, updated_at=$6, version=$7 WHERE id=$1 AND tenant_id=$2",
        )
        .bind(account.id.0).bind(account.tenant_id.0)
        .bind(account.balance).bind(account.available_balance)
        .bind(account.status.to_db_str()).bind(account.updated_at).bind(account.version)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn list_by_customer(&self, customer_id: CustomerId, tenant_id: TenantId) -> Result<Vec<Account>, DomainError> {
        let rows = sqlx::query(
            r#"SELECT id, tenant_id, account_number, customer_id, account_type::text, currency,
                      balance, available_balance, status::text, interest_rate, maturity_date,
                      created_at, updated_at, version
               FROM accounts WHERE customer_id = $1 AND tenant_id = $2"#,
        )
        .bind(customer_id.0).bind(tenant_id.0)
        .fetch_all(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(rows.iter().map(row_to_account).collect())
    }

    async fn list(&self, tenant_id: TenantId, page: PageRequest) -> Result<PageResponse<Account>, DomainError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts WHERE tenant_id = $1")
            .bind(tenant_id.0).fetch_one(&self.pool).await
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let rows = sqlx::query(
            r#"SELECT id, tenant_id, account_number, customer_id, account_type::text, currency,
                      balance, available_balance, status::text, interest_rate, maturity_date,
                      created_at, updated_at, version
               FROM accounts WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
        )
        .bind(tenant_id.0).bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let items = rows.iter().map(row_to_account).collect();
        Ok(PageResponse::new(items, total, &page))
    }
}

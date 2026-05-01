use async_trait::async_trait;
use domain::charge::{AppliedCharge, ChargeAppliedTo, ChargeDefinition, ChargeType};
use shared::errors::DomainError;
use shared::types::{Currency, TenantId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[async_trait]
pub trait ChargeRepository: Send + Sync {
    async fn create_definition(&self, def: &ChargeDefinition) -> Result<(), DomainError>;
    async fn find_definition(&self, id: Uuid, tenant_id: TenantId) -> Result<Option<ChargeDefinition>, DomainError>;
    async fn list_definitions(&self, tenant_id: TenantId) -> Result<Vec<ChargeDefinition>, DomainError>;
    async fn apply_charge(&self, charge: &AppliedCharge, tenant_id: TenantId) -> Result<(), DomainError>;
    async fn waive_charge(&self, id: Uuid, reason: String) -> Result<(), DomainError>;
}

pub struct PostgresChargeRepository { pool: PgPool }
impl PostgresChargeRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

fn parse_currency(s: &str) -> Currency {
    match s { "EUR" => Currency::EUR, "GBP" => Currency::GBP, "KES" => Currency::KES, _ => Currency::USD }
}

trait ToDbStr { fn to_db_str(&self) -> &'static str; }
impl ToDbStr for ChargeType {
    fn to_db_str(&self) -> &'static str {
        match self { ChargeType::Flat => "flat", ChargeType::Percentage => "percentage", ChargeType::Tiered => "tiered" }
    }
}
impl ToDbStr for ChargeAppliedTo {
    fn to_db_str(&self) -> &'static str {
        match self { ChargeAppliedTo::LoanOrigination => "loan_origination", ChargeAppliedTo::LoanRepayment => "loan_repayment",
            ChargeAppliedTo::AccountMaintenance => "account_maintenance", ChargeAppliedTo::Transfer => "transfer",
            ChargeAppliedTo::ATMWithdrawal => "atm_withdrawal" }
    }
}

fn row_to_def(r: &sqlx::postgres::PgRow) -> ChargeDefinition {
    ChargeDefinition {
        id: r.get("id"),
        tenant_id: TenantId(r.get::<Uuid, _>("tenant_id")),
        name: r.get("name"),
        charge_type: match r.get::<String, _>("charge_type").as_str() {
            "percentage" => ChargeType::Percentage, "tiered" => ChargeType::Tiered, _ => ChargeType::Flat,
        },
        applied_to: match r.get::<String, _>("applied_to").as_str() {
            "loan_repayment" => ChargeAppliedTo::LoanRepayment, "account_maintenance" => ChargeAppliedTo::AccountMaintenance,
            "transfer" => ChargeAppliedTo::Transfer, "atm_withdrawal" => ChargeAppliedTo::ATMWithdrawal,
            _ => ChargeAppliedTo::LoanOrigination,
        },
        amount: r.get("amount"),
        percentage: r.get("percentage"),
        currency: parse_currency(&r.get::<String, _>("currency")),
        is_active: r.get("is_active"),
    }
}

#[async_trait]
impl ChargeRepository for PostgresChargeRepository {
    async fn create_definition(&self, def: &ChargeDefinition) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO charge_definitions (id, tenant_id, name, charge_type, applied_to, amount, percentage, currency, is_active)
               VALUES ($1,$2,$3,$4::charge_type,$5,$6,$7,$8,$9)"#,
        )
        .bind(def.id).bind(def.tenant_id.0).bind(&def.name).bind(def.charge_type.to_db_str())
        .bind(def.applied_to.to_db_str()).bind(def.amount).bind(def.percentage)
        .bind(def.currency.to_string()).bind(def.is_active)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn find_definition(&self, id: Uuid, tenant_id: TenantId) -> Result<Option<ChargeDefinition>, DomainError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, name, charge_type::text, applied_to, amount, percentage, currency, is_active FROM charge_definitions WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id).bind(tenant_id.0)
        .fetch_optional(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_def))
    }

    async fn list_definitions(&self, tenant_id: TenantId) -> Result<Vec<ChargeDefinition>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, name, charge_type::text, applied_to, amount, percentage, currency, is_active FROM charge_definitions WHERE tenant_id = $1 AND is_active = true",
        )
        .bind(tenant_id.0).fetch_all(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(rows.iter().map(row_to_def).collect())
    }

    async fn apply_charge(&self, charge: &AppliedCharge, tenant_id: TenantId) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO applied_charges (id, tenant_id, charge_definition_id, account_id, loan_id, transaction_id, amount, currency, waived, applied_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)"#,
        )
        .bind(charge.id)
        .bind(tenant_id.0)
        .bind(charge.charge_definition_id)
        .bind(charge.account_id.map(|a| a.0))
        .bind(charge.loan_id.map(|l| l.0))
        .bind(charge.transaction_id.map(|t| t.0))
        .bind(charge.amount).bind(charge.currency.to_string())
        .bind(charge.waived).bind(charge.applied_at)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn waive_charge(&self, id: Uuid, reason: String) -> Result<(), DomainError> {
        sqlx::query("UPDATE applied_charges SET waived = true, waiver_reason = $2 WHERE id = $1")
        .bind(id).bind(reason)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }
}

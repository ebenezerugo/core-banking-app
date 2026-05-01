use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::loan::{InstallmentStatus, InterestType, Loan, LoanStatus, RepaymentScheduleEntry, RepaymentScheduleType};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::pagination::{PageRequest, PageResponse};
use shared::types::{AccountId, Currency, CustomerId, LoanId, TenantId};
use sqlx::{PgPool, Row};
use tracing::instrument;
use uuid::Uuid;

#[async_trait]
pub trait LoanRepository: Send + Sync {
    async fn create(&self, loan: &Loan) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: LoanId, tenant_id: TenantId) -> Result<Option<Loan>, DomainError>;
    async fn update(&self, loan: &Loan) -> Result<(), DomainError>;
    async fn list(&self, tenant_id: TenantId, page: PageRequest) -> Result<PageResponse<Loan>, DomainError>;
    async fn save_schedule(&self, entries: &[RepaymentScheduleEntry]) -> Result<(), DomainError>;
    async fn get_schedule(&self, loan_id: LoanId) -> Result<Vec<RepaymentScheduleEntry>, DomainError>;
    async fn update_schedule_entry(&self, entry: &RepaymentScheduleEntry) -> Result<(), DomainError>;
}

pub struct PostgresLoanRepository { pool: PgPool }
impl PostgresLoanRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

fn parse_currency(s: &str) -> Currency {
    match s { "EUR" => Currency::EUR, "GBP" => Currency::GBP, "KES" => Currency::KES,
              "NGN" => Currency::NGN, "GHS" => Currency::GHS, "ZAR" => Currency::ZAR,
              "UGX" => Currency::UGX, "TZS" => Currency::TZS, "RWF" => Currency::RWF, _ => Currency::USD }
}

fn row_to_loan(row: &sqlx::postgres::PgRow) -> Loan {
    Loan {
        id: LoanId(row.get::<Uuid, _>("id")),
        tenant_id: TenantId(row.get::<Uuid, _>("tenant_id")),
        customer_id: CustomerId(row.get::<Uuid, _>("customer_id")),
        account_id: AccountId(row.get::<Uuid, _>("account_id")),
        principal: row.get("principal"),
        currency: parse_currency(&row.get::<String, _>("currency")),
        interest_rate: row.get("interest_rate"),
        interest_type: match row.get::<String, _>("interest_type").as_str() { "compound" => InterestType::Compound, _ => InterestType::Simple },
        term_months: row.get("term_months"),
        repayment_schedule_type: match row.get::<String, _>("repayment_schedule_type").as_str() {
            "declining_balance" => RepaymentScheduleType::DecliningBalance,
            "annuity" => RepaymentScheduleType::Annuity,
            _ => RepaymentScheduleType::Flat,
        },
        disbursement_date: row.get("disbursement_date"),
        first_repayment_date: row.get("first_repayment_date"),
        maturity_date: row.get("maturity_date"),
        outstanding_principal: row.get("outstanding_principal"),
        outstanding_interest: row.get("outstanding_interest"),
        status: match row.get::<String, _>("status").as_str() {
            "submitted" => LoanStatus::Submitted, "under_review" => LoanStatus::UnderReview,
            "approved" => LoanStatus::Approved, "disbursed" => LoanStatus::Disbursed,
            "active" => LoanStatus::Active, "arrears" => LoanStatus::Arrears,
            "write_off" => LoanStatus::WriteOff, "closed" => LoanStatus::Closed,
            "rejected" => LoanStatus::Rejected, _ => LoanStatus::Draft,
        },
        arrears_days: row.get("arrears_days"),
        approved_by: row.get("approved_by"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        version: row.get("version"),
    }
}

trait ToDbStr { fn to_db_str(&self) -> &'static str; }
impl ToDbStr for LoanStatus {
    fn to_db_str(&self) -> &'static str {
        match self { LoanStatus::Draft => "draft", LoanStatus::Submitted => "submitted",
            LoanStatus::UnderReview => "under_review", LoanStatus::Approved => "approved",
            LoanStatus::Disbursed => "disbursed", LoanStatus::Active => "active",
            LoanStatus::Arrears => "arrears", LoanStatus::WriteOff => "write_off",
            LoanStatus::Closed => "closed", LoanStatus::Rejected => "rejected" }
    }
}
impl ToDbStr for InterestType {
    fn to_db_str(&self) -> &'static str { match self { InterestType::Simple => "simple", InterestType::Compound => "compound" } }
}
impl ToDbStr for RepaymentScheduleType {
    fn to_db_str(&self) -> &'static str {
        match self { RepaymentScheduleType::Flat => "flat", RepaymentScheduleType::DecliningBalance => "declining_balance", RepaymentScheduleType::Annuity => "annuity" }
    }
}
impl ToDbStr for InstallmentStatus {
    fn to_db_str(&self) -> &'static str {
        match self { InstallmentStatus::Pending => "pending", InstallmentStatus::PartiallyPaid => "partially_paid", InstallmentStatus::Paid => "paid", InstallmentStatus::Overdue => "overdue" }
    }
}

#[async_trait]
impl LoanRepository for PostgresLoanRepository {
    #[instrument(skip(self, loan))]
    async fn create(&self, loan: &Loan) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO loans (
                id, tenant_id, customer_id, account_id, principal, currency,
                interest_rate, interest_type, term_months, repayment_schedule_type,
                first_repayment_date, outstanding_principal, outstanding_interest,
                status, arrears_days, created_at, updated_at, version
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8::interest_type,$9,$10::repayment_schedule_type,$11,$12,$13,$14::loan_status,$15,$16,$17,$18)"#,
        )
        .bind(loan.id.0).bind(loan.tenant_id.0).bind(loan.customer_id.0).bind(loan.account_id.0)
        .bind(loan.principal).bind(loan.currency.to_string())
        .bind(loan.interest_rate).bind(loan.interest_type.to_db_str())
        .bind(loan.term_months).bind(loan.repayment_schedule_type.to_db_str())
        .bind(loan.first_repayment_date)
        .bind(loan.outstanding_principal).bind(loan.outstanding_interest)
        .bind(loan.status.to_db_str()).bind(loan.arrears_days)
        .bind(loan.created_at).bind(loan.updated_at).bind(loan.version)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn find_by_id(&self, id: LoanId, tenant_id: TenantId) -> Result<Option<Loan>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, customer_id, account_id, principal, currency,
                      interest_rate, interest_type::text, term_months, repayment_schedule_type::text,
                      disbursement_date, first_repayment_date, maturity_date,
                      outstanding_principal, outstanding_interest, status::text,
                      arrears_days, approved_by, created_at, updated_at, version
               FROM loans WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id.0).bind(tenant_id.0)
        .fetch_optional(&self.pool).await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_loan))
    }

    #[instrument(skip(self, loan))]
    async fn update(&self, loan: &Loan) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE loans SET outstanding_principal=$3, outstanding_interest=$4,
               status=$5::loan_status, arrears_days=$6, disbursement_date=$7, maturity_date=$8,
               approved_by=$9, updated_at=$10, version=$11
               WHERE id=$1 AND tenant_id=$2"#,
        )
        .bind(loan.id.0).bind(loan.tenant_id.0)
        .bind(loan.outstanding_principal).bind(loan.outstanding_interest)
        .bind(loan.status.to_db_str()).bind(loan.arrears_days)
        .bind(loan.disbursement_date).bind(loan.maturity_date)
        .bind(loan.approved_by).bind(loan.updated_at).bind(loan.version)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list(&self, tenant_id: TenantId, page: PageRequest) -> Result<PageResponse<Loan>, DomainError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM loans WHERE tenant_id = $1")
            .bind(tenant_id.0).fetch_one(&self.pool).await
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let rows = sqlx::query(
            r#"SELECT id, tenant_id, customer_id, account_id, principal, currency,
                      interest_rate, interest_type::text, term_months, repayment_schedule_type::text,
                      disbursement_date, first_repayment_date, maturity_date,
                      outstanding_principal, outstanding_interest, status::text,
                      arrears_days, approved_by, created_at, updated_at, version
               FROM loans WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
        )
        .bind(tenant_id.0).bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let items = rows.iter().map(row_to_loan).collect();
        Ok(PageResponse::new(items, total, &page))
    }

    async fn save_schedule(&self, entries: &[RepaymentScheduleEntry]) -> Result<(), DomainError> {
        for entry in entries {
            sqlx::query(
                r#"INSERT INTO loan_repayment_schedules
                   (id, loan_id, installment_number, due_date, principal_due, interest_due,
                    total_due, principal_paid, interest_paid, status)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::installment_status)"#,
            )
            .bind(entry.id).bind(entry.loan_id.0).bind(entry.installment_number)
            .bind(entry.due_date).bind(entry.principal_due).bind(entry.interest_due)
            .bind(entry.total_due).bind(entry.principal_paid).bind(entry.interest_paid)
            .bind(entry.status.to_db_str())
            .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        }
        Ok(())
    }

    async fn get_schedule(&self, loan_id: LoanId) -> Result<Vec<RepaymentScheduleEntry>, DomainError> {
        let rows = sqlx::query(
            r#"SELECT id, loan_id, installment_number, due_date, principal_due, interest_due,
                      total_due, principal_paid, interest_paid, status::text
               FROM loan_repayment_schedules WHERE loan_id = $1 ORDER BY installment_number"#,
        )
        .bind(loan_id.0).fetch_all(&self.pool).await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        Ok(rows.iter().map(|r| RepaymentScheduleEntry {
            id: r.get("id"),
            loan_id: LoanId(r.get::<Uuid, _>("loan_id")),
            installment_number: r.get("installment_number"),
            due_date: r.get("due_date"),
            principal_due: r.get("principal_due"),
            interest_due: r.get("interest_due"),
            total_due: r.get("total_due"),
            principal_paid: r.get("principal_paid"),
            interest_paid: r.get("interest_paid"),
            status: match r.get::<String, _>("status").as_str() {
                "partially_paid" => InstallmentStatus::PartiallyPaid,
                "paid" => InstallmentStatus::Paid,
                "overdue" => InstallmentStatus::Overdue,
                _ => InstallmentStatus::Pending,
            },
        }).collect())
    }

    async fn update_schedule_entry(&self, entry: &RepaymentScheduleEntry) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE loan_repayment_schedules SET principal_paid=$2, interest_paid=$3, status=$4::installment_status WHERE id=$1",
        )
        .bind(entry.id).bind(entry.principal_paid).bind(entry.interest_paid)
        .bind(entry.status.to_db_str())
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }
}

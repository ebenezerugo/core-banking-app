use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::types::{AccountId, Currency, CustomerId, LoanId, TenantId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoanStatus {
    Draft,
    Submitted,
    UnderReview,
    Approved,
    Disbursed,
    Active,
    Arrears,
    WriteOff,
    Closed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RepaymentScheduleType {
    Flat,
    DecliningBalance,
    Annuity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InterestType {
    Simple,
    Compound,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallmentStatus {
    Pending,
    PartiallyPaid,
    Paid,
    Overdue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loan {
    pub id: LoanId,
    pub tenant_id: TenantId,
    pub customer_id: CustomerId,
    pub account_id: AccountId,
    pub principal: Decimal,
    pub currency: Currency,
    /// Annual interest rate as a percentage (e.g. 12 means 12 % p.a.).
    pub interest_rate: Decimal,
    pub interest_type: InterestType,
    pub term_months: i32,
    pub repayment_schedule_type: RepaymentScheduleType,
    pub disbursement_date: Option<DateTime<Utc>>,
    pub first_repayment_date: Option<NaiveDate>,
    pub maturity_date: Option<NaiveDate>,
    pub outstanding_principal: Decimal,
    pub outstanding_interest: Decimal,
    pub status: LoanStatus,
    pub arrears_days: i32,
    pub approved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepaymentScheduleEntry {
    pub id: Uuid,
    pub loan_id: LoanId,
    pub installment_number: i32,
    pub due_date: NaiveDate,
    pub principal_due: Decimal,
    pub interest_due: Decimal,
    pub total_due: Decimal,
    pub principal_paid: Decimal,
    pub interest_paid: Decimal,
    pub status: InstallmentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoanEvent {
    LoanApplicationCreated { loan: Loan },
    LoanApproved { loan_id: LoanId, approved_by: Uuid, approved_at: DateTime<Utc> },
    LoanDisbursed { loan_id: LoanId, amount: Decimal, disbursed_at: DateTime<Utc> },
    RepaymentMade {
        loan_id: LoanId,
        amount: Decimal,
        principal_paid: Decimal,
        interest_paid: Decimal,
        made_at: DateTime<Utc>,
    },
    LoanMatured { loan_id: LoanId },
    LoanWrittenOff { loan_id: LoanId, reason: String, written_off_at: DateTime<Utc> },
    LoanRestructured { loan_id: LoanId, new_terms: serde_json::Value },
}

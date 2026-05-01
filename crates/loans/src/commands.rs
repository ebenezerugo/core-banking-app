use chrono::NaiveDate;
use rust_decimal::Decimal;
use shared::types::{AccountId, CustomerId, LoanId, TenantId};
use domain::loan::{InterestType, RepaymentScheduleType};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateLoanApplicationCommand {
    pub tenant_id: TenantId,
    pub customer_id: CustomerId,
    pub account_id: AccountId,
    pub principal: Decimal,
    pub interest_rate: Decimal,
    pub interest_type: InterestType,
    pub term_months: i32,
    pub repayment_schedule_type: RepaymentScheduleType,
    pub first_repayment_date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct ApproveLoanCommand {
    pub loan_id: LoanId,
    pub tenant_id: TenantId,
    pub approved_by: Uuid,
}

#[derive(Debug, Clone)]
pub struct DisburseLoanCommand {
    pub loan_id: LoanId,
    pub tenant_id: TenantId,
    pub disbursed_by: Uuid,
}

#[derive(Debug, Clone)]
pub struct ProcessRepaymentCommand {
    pub loan_id: LoanId,
    pub tenant_id: TenantId,
    pub amount: Decimal,
}

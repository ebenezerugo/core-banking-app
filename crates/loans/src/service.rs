use std::sync::Arc;

use chrono::{Utc, NaiveDate};
use rust_decimal::Decimal;
use domain::loan::{
    InstallmentStatus, Loan, LoanEvent, LoanStatus, RepaymentScheduleEntry,
};
use shared::errors::DomainError;
use shared::events::{EventEnvelope, EventStore};
use shared::types::{LoanId, TenantId};
use tracing::instrument;
use uuid::Uuid;

use crate::amortization::AmortizationEngine;
use crate::commands::{
    ApproveLoanCommand, CreateLoanApplicationCommand, DisburseLoanCommand,
    ProcessRepaymentCommand,
};
use crate::repository::LoanRepository;

pub struct LoanService<R: LoanRepository> {
    repository: R,
    event_store: Arc<dyn EventStore>,
}

impl<R: LoanRepository> LoanService<R> {
    pub fn new(repository: R, event_store: Arc<dyn EventStore>) -> Self {
        Self { repository, event_store }
    }

    #[instrument(skip(self, cmd))]
    pub async fn create_application(
        &self,
        cmd: CreateLoanApplicationCommand,
    ) -> Result<Loan, DomainError> {
        if cmd.principal <= Decimal::ZERO {
            return Err(DomainError::ValidationError("Principal must be positive".into()));
        }
        if cmd.term_months <= 0 {
            return Err(DomainError::ValidationError("Term must be positive".into()));
        }

        let loan = Loan {
            id: LoanId::new(),
            tenant_id: cmd.tenant_id,
            customer_id: cmd.customer_id,
            account_id: cmd.account_id,
            principal: cmd.principal,
            currency: shared::types::Currency::USD,
            interest_rate: cmd.interest_rate,
            interest_type: cmd.interest_type,
            term_months: cmd.term_months,
            repayment_schedule_type: cmd.repayment_schedule_type.clone(),
            disbursement_date: None,
            first_repayment_date: Some(cmd.first_repayment_date),
            maturity_date: None,
            outstanding_principal: cmd.principal,
            outstanding_interest: Decimal::ZERO,
            status: LoanStatus::Draft,
            arrears_days: 0,
            approved_by: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 0,
        };

        self.repository.create(&loan).await?;

        // Generate and store repayment schedule
        let schedule = AmortizationEngine::generate_schedule(
            cmd.principal,
            cmd.interest_rate,
            cmd.term_months,
            cmd.first_repayment_date,
            cmd.repayment_schedule_type,
        );

        let entries: Vec<RepaymentScheduleEntry> = schedule
            .into_iter()
            .map(|e| RepaymentScheduleEntry {
                id: Uuid::new_v4(),
                loan_id: loan.id,
                installment_number: e.installment_number,
                due_date: e.due_date,
                principal_due: e.principal_due,
                interest_due: e.interest_due,
                total_due: e.total_due,
                principal_paid: Decimal::ZERO,
                interest_paid: Decimal::ZERO,
                status: InstallmentStatus::Pending,
            })
            .collect();

        self.repository.save_schedule(&entries).await?;

        let event = LoanEvent::LoanApplicationCreated { loan: loan.clone() };
        let _ = self.event_store.append(EventEnvelope::new(
            loan.id.0,
            "Loan",
            "LoanApplicationCreated",
            serde_json::to_value(&event).unwrap_or_default(),
        )).await;

        Ok(loan)
    }

    #[instrument(skip(self, cmd))]
    pub async fn approve_loan(&self, cmd: ApproveLoanCommand) -> Result<Loan, DomainError> {
        let mut loan = self
            .repository
            .find_by_id(cmd.loan_id, cmd.tenant_id)
            .await?
            .ok_or_else(|| DomainError::LoanNotFound(cmd.loan_id.to_string()))?;

        if loan.status != LoanStatus::Draft && loan.status != LoanStatus::Submitted {
            return Err(DomainError::InvalidStateTransition(
                format!("Cannot approve loan in state {:?}", loan.status),
            ));
        }

        loan.status = LoanStatus::Approved;
        loan.approved_by = Some(cmd.approved_by);
        loan.updated_at = Utc::now();
        loan.version += 1;

        self.repository.update(&loan).await?;

        let event = LoanEvent::LoanApproved {
            loan_id: loan.id,
            approved_by: cmd.approved_by,
            approved_at: Utc::now(),
        };
        let _ = self.event_store.append(EventEnvelope::new(
            loan.id.0,
            "Loan",
            "LoanApproved",
            serde_json::to_value(&event).unwrap_or_default(),
        )).await;

        Ok(loan)
    }

    #[instrument(skip(self, cmd))]
    pub async fn disburse_loan(&self, cmd: DisburseLoanCommand) -> Result<Loan, DomainError> {
        let mut loan = self
            .repository
            .find_by_id(cmd.loan_id, cmd.tenant_id)
            .await?
            .ok_or_else(|| DomainError::LoanNotFound(cmd.loan_id.to_string()))?;

        if loan.status != LoanStatus::Approved {
            return Err(DomainError::InvalidStateTransition(
                format!("Cannot disburse loan in state {:?}", loan.status),
            ));
        }

        let now = Utc::now();
        loan.status = LoanStatus::Active;
        loan.disbursement_date = Some(now);
        loan.updated_at = now;
        loan.version += 1;

        self.repository.update(&loan).await?;

        let event = LoanEvent::LoanDisbursed {
            loan_id: loan.id,
            amount: loan.principal,
            disbursed_at: now,
        };
        let _ = self.event_store.append(EventEnvelope::new(
            loan.id.0,
            "Loan",
            "LoanDisbursed",
            serde_json::to_value(&event).unwrap_or_default(),
        )).await;

        Ok(loan)
    }

    #[instrument(skip(self, cmd))]
    pub async fn process_repayment(
        &self,
        cmd: ProcessRepaymentCommand,
    ) -> Result<Loan, DomainError> {
        let mut loan = self
            .repository
            .find_by_id(cmd.loan_id, cmd.tenant_id)
            .await?
            .ok_or_else(|| DomainError::LoanNotFound(cmd.loan_id.to_string()))?;

        if loan.status != LoanStatus::Active && loan.status != LoanStatus::Arrears {
            return Err(DomainError::InvalidStateTransition(
                format!("Cannot repay loan in state {:?}", loan.status),
            ));
        }

        // Apply to interest first, then principal
        let mut remaining = cmd.amount;
        let interest_payment = remaining.min(loan.outstanding_interest);
        remaining -= interest_payment;
        let principal_payment = remaining.min(loan.outstanding_principal);

        loan.outstanding_interest -= interest_payment;
        loan.outstanding_principal -= principal_payment;

        if loan.outstanding_principal <= Decimal::ZERO
            && loan.outstanding_interest <= Decimal::ZERO
        {
            loan.status = LoanStatus::Closed;
        }

        loan.updated_at = Utc::now();
        loan.version += 1;
        self.repository.update(&loan).await?;

        let event = LoanEvent::RepaymentMade {
            loan_id: loan.id,
            amount: cmd.amount,
            principal_paid: principal_payment,
            interest_paid: interest_payment,
            made_at: Utc::now(),
        };
        let _ = self.event_store.append(EventEnvelope::new(
            loan.id.0,
            "Loan",
            "RepaymentMade",
            serde_json::to_value(&event).unwrap_or_default(),
        )).await;

        Ok(loan)
    }

    pub async fn get_loan(&self, id: LoanId, tenant_id: TenantId) -> Result<Loan, DomainError> {
        self.repository
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::LoanNotFound(id.to_string()))
    }

    pub async fn get_schedule(
        &self,
        loan_id: LoanId,
        _tenant_id: TenantId,
    ) -> Result<Vec<RepaymentScheduleEntry>, DomainError> {
        self.repository.get_schedule(loan_id).await
    }
}

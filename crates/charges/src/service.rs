use chrono::Utc;
use domain::charge::{AppliedCharge, ChargeDefinition, ChargeType};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::types::{AccountId, LoanId, TenantId, TransactionId};
use tracing::instrument;
use uuid::Uuid;

use crate::repository::ChargeRepository;

pub struct ChargeService<R: ChargeRepository> {
    repository: R,
}

impl<R: ChargeRepository> ChargeService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create_definition(
        &self,
        def: ChargeDefinition,
    ) -> Result<ChargeDefinition, DomainError> {
        self.repository.create_definition(&def).await?;
        Ok(def)
    }

    #[instrument(skip(self))]
    pub async fn calculate_charge(
        &self,
        definition_id: Uuid,
        tenant_id: TenantId,
        base_amount: Decimal,
    ) -> Result<Decimal, DomainError> {
        let def = self
            .repository
            .find_definition(definition_id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(definition_id.to_string()))?;

        let charge_amount = match def.charge_type {
            ChargeType::Flat => def.amount,
            ChargeType::Percentage => (base_amount * def.percentage / Decimal::from(100)).round_dp(2),
            ChargeType::Tiered => def.amount, // Simplified: tiered uses flat amount
        };
        Ok(charge_amount)
    }

    #[instrument(skip(self))]
    pub async fn apply_charge(
        &self,
        definition_id: Uuid,
        tenant_id: TenantId,
        base_amount: Decimal,
        account_id: Option<AccountId>,
        loan_id: Option<LoanId>,
        transaction_id: Option<TransactionId>,
    ) -> Result<AppliedCharge, DomainError> {
        let amount = self.calculate_charge(definition_id, tenant_id, base_amount).await?;

        let def = self
            .repository
            .find_definition(definition_id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(definition_id.to_string()))?;

        let charge = AppliedCharge {
            id: Uuid::new_v4(),
            charge_definition_id: definition_id,
            account_id,
            loan_id,
            transaction_id,
            amount,
            currency: def.currency,
            waived: false,
            waiver_reason: None,
            applied_at: Utc::now(),
        };

        self.repository.apply_charge(&charge, tenant_id).await?;
        Ok(charge)
    }

    #[instrument(skip(self))]
    pub async fn waive_charge(
        &self,
        charge_id: Uuid,
        reason: String,
    ) -> Result<(), DomainError> {
        self.repository.waive_charge(charge_id, reason).await
    }

    pub async fn list_definitions(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<ChargeDefinition>, DomainError> {
        self.repository.list_definitions(tenant_id).await
    }
}

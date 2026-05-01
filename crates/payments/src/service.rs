use chrono::Utc;
use domain::payment::{Payment, PaymentChannel, PaymentStatus};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::types::{Currency, TenantId, TransactionId};
use tracing::instrument;
use uuid::Uuid;

use crate::repository::PaymentRepository;

pub struct PaymentService<R: PaymentRepository> {
    repository: R,
}

impl<R: PaymentRepository> PaymentService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    #[instrument(skip(self))]
    pub async fn initiate_payment(
        &self,
        tenant_id: TenantId,
        transaction_id: TransactionId,
        channel: PaymentChannel,
        sender_account: String,
        receiver_account: String,
        amount: Decimal,
        currency: Currency,
        routing_number: Option<String>,
        swift_code: Option<String>,
        reference: String,
    ) -> Result<Payment, DomainError> {
        let payment = Payment {
            id: Uuid::new_v4(),
            tenant_id,
            transaction_id,
            channel,
            sender_account,
            receiver_account,
            amount,
            currency,
            status: PaymentStatus::Initiated,
            routing_number,
            swift_code,
            reference,
            initiated_at: Utc::now(),
            settled_at: None,
        };

        self.repository.create(&payment).await?;
        Ok(payment)
    }

    #[instrument(skip(self))]
    pub async fn settle_payment(
        &self,
        payment_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<(), DomainError> {
        let payment = self
            .repository
            .find_by_id(payment_id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(payment_id.to_string()))?;

        if payment.status != PaymentStatus::Initiated && payment.status != PaymentStatus::Processing {
            return Err(DomainError::InvalidStateTransition(
                format!("Cannot settle payment in state {:?}", payment.status),
            ));
        }

        self.repository
            .update_status(payment_id, PaymentStatus::Settled, Some(Utc::now()))
            .await
    }

    #[instrument(skip(self))]
    pub async fn fail_payment(
        &self,
        payment_id: Uuid,
        tenant_id: TenantId,
    ) -> Result<(), DomainError> {
        self.repository
            .update_status(payment_id, PaymentStatus::Failed, None)
            .await
    }

    pub async fn get_payment(
        &self,
        id: Uuid,
        tenant_id: TenantId,
    ) -> Result<Payment, DomainError> {
        self.repository
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(id.to_string()))
    }
}

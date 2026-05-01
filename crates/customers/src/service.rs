use std::sync::Arc;

use chrono::Utc;
use domain::customer::{Customer, CustomerEvent, CustomerStatus, KycStatus};
use shared::errors::DomainError;
use shared::events::{EventEnvelope, EventStore};
use shared::types::{CustomerId, TenantId};
use tracing::instrument;
use uuid::Uuid;

use crate::commands::{CreateCustomerCommand, UpdateCustomerCommand, SubmitKycCommand};
use crate::repository::CustomerRepository;

pub struct CustomerService<R: CustomerRepository> {
    repository: R,
    event_store: Arc<dyn EventStore>,
}

impl<R: CustomerRepository> CustomerService<R> {
    pub fn new(repository: R, event_store: Arc<dyn EventStore>) -> Self {
        Self { repository, event_store }
    }

    #[instrument(skip(self, cmd))]
    pub async fn create_customer(
        &self,
        cmd: CreateCustomerCommand,
    ) -> Result<Customer, DomainError> {
        if cmd.email.is_empty() {
            return Err(DomainError::ValidationError("Email required".into()));
        }

        if self
            .repository
            .find_by_email(&cmd.email, cmd.tenant_id)
            .await?
            .is_some()
        {
            return Err(DomainError::ValidationError(
                "Email already registered".into(),
            ));
        }

        let customer = Customer {
            id: CustomerId::new(),
            tenant_id: cmd.tenant_id,
            customer_type: cmd.customer_type,
            first_name: cmd.first_name,
            last_name: cmd.last_name,
            business_name: cmd.business_name,
            email: cmd.email,
            phone: cmd.phone,
            date_of_birth: cmd.date_of_birth,
            national_id: cmd.national_id,
            address: cmd.address,
            status: CustomerStatus::Pending,
            kyc_status: KycStatus::NotStarted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.repository.create(&customer).await?;

        let event = CustomerEvent::CustomerCreated { customer: customer.clone() };
        let envelope = EventEnvelope::new(
            customer.id.0,
            "Customer",
            "CustomerCreated",
            serde_json::to_value(&event).unwrap_or_default(),
        );
        let _ = self.event_store.append(envelope).await;

        Ok(customer)
    }

    #[instrument(skip(self, cmd))]
    pub async fn update_customer(
        &self,
        cmd: UpdateCustomerCommand,
    ) -> Result<Customer, DomainError> {
        let mut customer = self
            .repository
            .find_by_id(cmd.customer_id, cmd.tenant_id)
            .await?
            .ok_or_else(|| DomainError::CustomerNotFound(cmd.customer_id.to_string()))?;

        if let Some(v) = cmd.first_name { customer.first_name = Some(v); }
        if let Some(v) = cmd.last_name  { customer.last_name  = Some(v); }
        if let Some(v) = cmd.business_name { customer.business_name = Some(v); }
        if let Some(v) = cmd.phone     { customer.phone = v; }
        if let Some(v) = cmd.address   { customer.address = v; }
        customer.updated_at = Utc::now();

        self.repository.update(&customer).await?;

        let changes = serde_json::json!({ "updated_at": customer.updated_at });
        let event = CustomerEvent::CustomerUpdated { customer_id: customer.id, changes };
        let envelope = EventEnvelope::new(
            customer.id.0,
            "Customer",
            "CustomerUpdated",
            serde_json::to_value(&event).unwrap_or_default(),
        );
        let _ = self.event_store.append(envelope).await;

        Ok(customer)
    }

    #[instrument(skip(self, cmd))]
    pub async fn submit_kyc(&self, cmd: SubmitKycCommand) -> Result<(), DomainError> {
        let customer = self
            .repository
            .find_by_id(cmd.customer_id, cmd.tenant_id)
            .await?
            .ok_or_else(|| DomainError::CustomerNotFound(cmd.customer_id.to_string()))?;

        let old_status = customer.kyc_status.clone();
        self.repository
            .update_kyc_status(cmd.customer_id, cmd.tenant_id, cmd.new_status.clone())
            .await?;

        let event = CustomerEvent::KycStatusChanged {
            customer_id: cmd.customer_id,
            old_status,
            new_status: cmd.new_status,
        };
        let envelope = EventEnvelope::new(
            cmd.customer_id.0,
            "Customer",
            "KycStatusChanged",
            serde_json::to_value(&event).unwrap_or_default(),
        );
        let _ = self.event_store.append(envelope).await;

        Ok(())
    }

    pub async fn get_customer(
        &self,
        id: CustomerId,
        tenant_id: TenantId,
    ) -> Result<Customer, DomainError> {
        self.repository
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| DomainError::CustomerNotFound(id.to_string()))
    }
}



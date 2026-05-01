use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shared::types::{CustomerId, TenantId};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CustomerStatus {
    Pending,
    Active,
    Suspended,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CustomerType {
    Individual,
    Business,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KycStatus {
    NotStarted,
    InProgress,
    Verified,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub country: String,
    pub postal_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: CustomerId,
    pub tenant_id: TenantId,
    pub customer_type: CustomerType,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub business_name: Option<String>,
    pub email: String,
    pub phone: String,
    pub date_of_birth: Option<NaiveDate>,
    pub national_id: Option<String>,
    pub address: Address,
    pub status: CustomerStatus,
    pub kyc_status: KycStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CustomerEvent {
    CustomerCreated { customer: Customer },
    CustomerUpdated { customer_id: CustomerId, changes: serde_json::Value },
    KycStatusChanged { customer_id: CustomerId, old_status: KycStatus, new_status: KycStatus },
    CustomerSuspended { customer_id: CustomerId, reason: String },
    CustomerClosed { customer_id: CustomerId, reason: String },
}

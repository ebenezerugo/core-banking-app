use chrono::NaiveDate;
use domain::customer::{Address, CustomerType};
use shared::types::{CustomerId, TenantId};
use domain::customer::KycStatus;

#[derive(Debug, Clone)]
pub struct CreateCustomerCommand {
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
}

#[derive(Debug, Clone)]
pub struct UpdateCustomerCommand {
    pub customer_id: CustomerId,
    pub tenant_id: TenantId,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub business_name: Option<String>,
    pub phone: Option<String>,
    pub address: Option<Address>,
}

#[derive(Debug, Clone)]
pub struct SubmitKycCommand {
    pub customer_id: CustomerId,
    pub tenant_id: TenantId,
    pub new_status: KycStatus,
    pub document_reference: Option<String>,
}

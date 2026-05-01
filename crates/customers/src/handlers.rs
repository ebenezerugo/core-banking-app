use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use domain::customer::{Address, CustomerType, KycStatus};
use shared::pagination::PageRequest;
use shared::types::{CustomerId, TenantId};

use crate::commands::{CreateCustomerCommand, SubmitKycCommand, UpdateCustomerCommand};
use crate::repository::PostgresCustomerRepository;
use crate::service::CustomerService;
use shared::events::EventStore;

pub type CustomerServiceState =
    Arc<CustomerService<PostgresCustomerRepository>>;

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub customer_type: CustomerType,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub business_name: Option<String>,
    pub email: String,
    pub phone: String,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub national_id: Option<String>,
    pub address: Address,
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct KycUpdateRequest {
    pub new_status: KycStatus,
    pub document_reference: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

pub async fn create_customer(
    State(svc): State<CustomerServiceState>,
    Json(req): Json<CreateCustomerRequest>,
) -> impl IntoResponse {
    let cmd = CreateCustomerCommand {
        tenant_id: TenantId(req.tenant_id),
        customer_type: req.customer_type,
        first_name: req.first_name,
        last_name: req.last_name,
        business_name: req.business_name,
        email: req.email,
        phone: req.phone,
        date_of_birth: req.date_of_birth,
        national_id: req.national_id,
        address: req.address,
    };
    match svc.create_customer(cmd).await {
        Ok(c) => (StatusCode::CREATED, Json(serde_json::json!(c))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: e.to_string() }),
        )
            .into_response(),
    }
}

pub async fn get_customer(
    State(svc): State<CustomerServiceState>,
    Path((tenant_id, customer_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match svc
        .get_customer(CustomerId(customer_id), TenantId(tenant_id))
        .await
    {
        Ok(c) => (StatusCode::OK, Json(serde_json::json!(c))).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ApiError { error: e.to_string() }),
        )
            .into_response(),
    }
}

pub async fn update_kyc(
    State(svc): State<CustomerServiceState>,
    Path((tenant_id, customer_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<KycUpdateRequest>,
) -> impl IntoResponse {
    let cmd = SubmitKycCommand {
        customer_id: CustomerId(customer_id),
        tenant_id: TenantId(tenant_id),
        new_status: req.new_status,
        document_reference: req.document_reference,
    };
    match svc.submit_kyc(cmd).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError { error: e.to_string() }),
        )
            .into_response(),
    }
}

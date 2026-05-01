use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use domain::customer::{Address, Customer, CustomerStatus, CustomerType, KycStatus};
use shared::errors::DomainError;
use shared::pagination::{PageRequest, PageResponse};
use shared::types::{CustomerId, TenantId};
use sqlx::{PgPool, Row};
use tracing::instrument;
use uuid::Uuid;

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn create(&self, customer: &Customer) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: CustomerId, tenant_id: TenantId) -> Result<Option<Customer>, DomainError>;
    async fn find_by_email(&self, email: &str, tenant_id: TenantId) -> Result<Option<Customer>, DomainError>;
    async fn update(&self, customer: &Customer) -> Result<(), DomainError>;
    async fn list(&self, tenant_id: TenantId, page: PageRequest) -> Result<PageResponse<Customer>, DomainError>;
    async fn update_kyc_status(&self, customer_id: CustomerId, tenant_id: TenantId, status: KycStatus) -> Result<(), DomainError>;
}

pub struct PostgresCustomerRepository {
    pool: PgPool,
}

impl PostgresCustomerRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

fn row_to_customer(row: &sqlx::postgres::PgRow) -> Customer {
    use sqlx::Row;
    Customer {
        id: CustomerId(row.get::<Uuid, _>("id")),
        tenant_id: TenantId(row.get::<Uuid, _>("tenant_id")),
        customer_type: match row.get::<String, _>("customer_type").as_str() {
            "business" => CustomerType::Business,
            "group" => CustomerType::Group,
            _ => CustomerType::Individual,
        },
        first_name: row.get("first_name"),
        last_name: row.get("last_name"),
        business_name: row.get("business_name"),
        email: row.get("email"),
        phone: row.get("phone"),
        date_of_birth: row.get("date_of_birth"),
        national_id: row.get("national_id"),
        address: Address {
            street: row.get("street"),
            city: row.get("city"),
            state: row.get("state"),
            country: row.get("country"),
            postal_code: row.get("postal_code"),
        },
        status: match row.get::<String, _>("status").as_str() {
            "active" => CustomerStatus::Active,
            "suspended" => CustomerStatus::Suspended,
            "closed" => CustomerStatus::Closed,
            _ => CustomerStatus::Pending,
        },
        kyc_status: match row.get::<String, _>("kyc_status").as_str() {
            "in_progress" => KycStatus::InProgress,
            "verified" => KycStatus::Verified,
            "rejected" => KycStatus::Rejected,
            "expired" => KycStatus::Expired,
            _ => KycStatus::NotStarted,
        },
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

#[async_trait]
impl CustomerRepository for PostgresCustomerRepository {
    #[instrument(skip(self, customer))]
    async fn create(&self, customer: &Customer) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO customers (
                id, tenant_id, customer_type, first_name, last_name, business_name,
                email, phone, date_of_birth, national_id,
                street, city, state, country, postal_code,
                status, kyc_status, created_at, updated_at
            ) VALUES (
                $1, $2, $3::customer_type, $4, $5, $6,
                $7, $8, $9, $10,
                $11, $12, $13, $14, $15,
                $16::customer_status, $17::kyc_status, $18, $19
            )"#,
        )
        .bind(customer.id.0)
        .bind(customer.tenant_id.0)
        .bind(customer.customer_type.to_db_str())
        .bind(&customer.first_name)
        .bind(&customer.last_name)
        .bind(&customer.business_name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(customer.date_of_birth)
        .bind(&customer.national_id)
        .bind(&customer.address.street)
        .bind(&customer.address.city)
        .bind(&customer.address.state)
        .bind(&customer.address.country)
        .bind(&customer.address.postal_code)
        .bind(customer.status.to_db_str())
        .bind(customer.kyc_status.to_db_str())
        .bind(customer.created_at)
        .bind(customer.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn find_by_id(&self, id: CustomerId, tenant_id: TenantId) -> Result<Option<Customer>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, customer_type::text, first_name, last_name, business_name,
                      email, phone, date_of_birth, national_id,
                      street, city, state, country, postal_code,
                      status::text, kyc_status::text, created_at, updated_at
               FROM customers WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id.0)
        .bind(tenant_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_customer))
    }

    #[instrument(skip(self))]
    async fn find_by_email(&self, email: &str, tenant_id: TenantId) -> Result<Option<Customer>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, customer_type::text, first_name, last_name, business_name,
                      email, phone, date_of_birth, national_id,
                      street, city, state, country, postal_code,
                      status::text, kyc_status::text, created_at, updated_at
               FROM customers WHERE email = $1 AND tenant_id = $2"#,
        )
        .bind(email)
        .bind(tenant_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_customer))
    }

    #[instrument(skip(self, customer))]
    async fn update(&self, customer: &Customer) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE customers SET
               first_name = $3, last_name = $4, business_name = $5,
               phone = $6, street = $7, city = $8, state = $9,
               country = $10, postal_code = $11, updated_at = $12
               WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(customer.id.0)
        .bind(customer.tenant_id.0)
        .bind(&customer.first_name)
        .bind(&customer.last_name)
        .bind(&customer.business_name)
        .bind(&customer.phone)
        .bind(&customer.address.street)
        .bind(&customer.address.city)
        .bind(&customer.address.state)
        .bind(&customer.address.country)
        .bind(&customer.address.postal_code)
        .bind(customer.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn list(&self, tenant_id: TenantId, page: PageRequest) -> Result<PageResponse<Customer>, DomainError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM customers WHERE tenant_id = $1")
            .bind(tenant_id.0)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let rows = sqlx::query(
            r#"SELECT id, tenant_id, customer_type::text, first_name, last_name, business_name,
                      email, phone, date_of_birth, national_id,
                      street, city, state, country, postal_code,
                      status::text, kyc_status::text, created_at, updated_at
               FROM customers WHERE tenant_id = $1
               ORDER BY created_at DESC LIMIT $2 OFFSET $3"#,
        )
        .bind(tenant_id.0)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let items = rows.iter().map(row_to_customer).collect();
        Ok(PageResponse::new(items, total, &page))
    }

    #[instrument(skip(self))]
    async fn update_kyc_status(&self, customer_id: CustomerId, tenant_id: TenantId, status: KycStatus) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE customers SET kyc_status = $3::kyc_status, updated_at = NOW()
               WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(customer_id.0)
        .bind(tenant_id.0)
        .bind(status.to_db_str())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }
}

trait ToDbStr { fn to_db_str(&self) -> &'static str; }

impl ToDbStr for CustomerType {
    fn to_db_str(&self) -> &'static str {
        match self { CustomerType::Individual => "individual", CustomerType::Business => "business", CustomerType::Group => "group" }
    }
}
impl ToDbStr for CustomerStatus {
    fn to_db_str(&self) -> &'static str {
        match self { CustomerStatus::Pending => "pending", CustomerStatus::Active => "active", CustomerStatus::Suspended => "suspended", CustomerStatus::Closed => "closed" }
    }
}
impl ToDbStr for KycStatus {
    fn to_db_str(&self) -> &'static str {
        match self { KycStatus::NotStarted => "not_started", KycStatus::InProgress => "in_progress", KycStatus::Verified => "verified", KycStatus::Rejected => "rejected", KycStatus::Expired => "expired" }
    }
}

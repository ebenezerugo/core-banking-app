use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::payment::{Payment, PaymentChannel, PaymentStatus};
use rust_decimal::Decimal;
use shared::errors::DomainError;
use shared::types::{Currency, TenantId, TransactionId};
use sqlx::{PgPool, Row};
use tracing::instrument;
use uuid::Uuid;

#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn create(&self, payment: &Payment) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: Uuid, tenant_id: TenantId) -> Result<Option<Payment>, DomainError>;
    async fn update_status(&self, id: Uuid, status: PaymentStatus, settled_at: Option<DateTime<Utc>>) -> Result<(), DomainError>;
    async fn find_by_reference(&self, reference: &str, tenant_id: TenantId) -> Result<Option<Payment>, DomainError>;
}

pub struct PostgresPaymentRepository { pool: PgPool }
impl PostgresPaymentRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

fn parse_currency(s: &str) -> Currency {
    match s { "EUR" => Currency::EUR, "GBP" => Currency::GBP, "KES" => Currency::KES, _ => Currency::USD }
}
fn parse_channel(s: &str) -> PaymentChannel {
    match s { "swift" => PaymentChannel::SWIFT, "internal_transfer" => PaymentChannel::InternalTransfer,
              "mobile_money" => PaymentChannel::MobileMoney, "card" => PaymentChannel::Card, _ => PaymentChannel::ACH }
}
fn parse_status(s: &str) -> PaymentStatus {
    match s { "processing" => PaymentStatus::Processing, "settled" => PaymentStatus::Settled,
              "failed" => PaymentStatus::Failed, "rejected" => PaymentStatus::Rejected, _ => PaymentStatus::Initiated }
}

trait ToDbStr { fn to_db_str(&self) -> &'static str; }
impl ToDbStr for PaymentChannel {
    fn to_db_str(&self) -> &'static str {
        match self { PaymentChannel::ACH => "ach", PaymentChannel::SWIFT => "swift",
            PaymentChannel::InternalTransfer => "internal_transfer", PaymentChannel::MobileMoney => "mobile_money",
            PaymentChannel::Card => "card" }
    }
}
impl ToDbStr for PaymentStatus {
    fn to_db_str(&self) -> &'static str {
        match self { PaymentStatus::Initiated => "initiated", PaymentStatus::Processing => "processing",
            PaymentStatus::Settled => "settled", PaymentStatus::Failed => "failed", PaymentStatus::Rejected => "rejected" }
    }
}

fn row_to_payment(r: &sqlx::postgres::PgRow) -> Payment {
    Payment {
        id: r.get("id"),
        tenant_id: TenantId(r.get::<Uuid, _>("tenant_id")),
        transaction_id: TransactionId(r.get::<Uuid, _>("transaction_id")),
        channel: parse_channel(&r.get::<String, _>("channel")),
        sender_account: r.get("sender_account"),
        receiver_account: r.get("receiver_account"),
        amount: r.get("amount"),
        currency: parse_currency(&r.get::<String, _>("currency")),
        status: parse_status(&r.get::<String, _>("status")),
        routing_number: r.get("routing_number"),
        swift_code: r.get("swift_code"),
        reference: r.get("reference"),
        initiated_at: r.get("initiated_at"),
        settled_at: r.get("settled_at"),
    }
}

#[async_trait]
impl PaymentRepository for PostgresPaymentRepository {
    async fn create(&self, payment: &Payment) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO payments (id, tenant_id, transaction_id, channel, sender_account, receiver_account,
               amount, currency, status, routing_number, swift_code, reference, initiated_at)
               VALUES ($1,$2,$3,$4::payment_channel,$5,$6,$7,$8,$9::payment_status,$10,$11,$12,$13)"#,
        )
        .bind(payment.id).bind(payment.tenant_id.0).bind(payment.transaction_id.0)
        .bind(payment.channel.to_db_str())
        .bind(&payment.sender_account).bind(&payment.receiver_account)
        .bind(payment.amount).bind(payment.currency.to_string())
        .bind(payment.status.to_db_str())
        .bind(&payment.routing_number).bind(&payment.swift_code).bind(&payment.reference)
        .bind(payment.initiated_at)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid, tenant_id: TenantId) -> Result<Option<Payment>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, transaction_id, channel::text, sender_account, receiver_account,
                      amount, currency, status::text, routing_number, swift_code, reference, initiated_at, settled_at
               FROM payments WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id).bind(tenant_id.0)
        .fetch_optional(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_payment))
    }

    async fn update_status(&self, id: Uuid, status: PaymentStatus, settled_at: Option<DateTime<Utc>>) -> Result<(), DomainError> {
        sqlx::query("UPDATE payments SET status = $2::payment_status, settled_at = $3 WHERE id = $1")
        .bind(id).bind(status.to_db_str()).bind(settled_at)
        .execute(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(())
    }

    async fn find_by_reference(&self, reference: &str, tenant_id: TenantId) -> Result<Option<Payment>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, tenant_id, transaction_id, channel::text, sender_account, receiver_account,
                      amount, currency, status::text, routing_number, swift_code, reference, initiated_at, settled_at
               FROM payments WHERE reference = $1 AND tenant_id = $2"#,
        )
        .bind(reference).bind(tenant_id.0)
        .fetch_optional(&self.pool).await.map_err(|e| DomainError::ValidationError(e.to_string()))?;
        Ok(row.as_ref().map(row_to_payment))
    }
}

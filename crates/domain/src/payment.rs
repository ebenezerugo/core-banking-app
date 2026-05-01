use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::types::{Currency, TenantId, TransactionId};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentChannel {
    ACH,
    SWIFT,
    InternalTransfer,
    MobileMoney,
    Card,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Initiated,
    Processing,
    Settled,
    Failed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub transaction_id: TransactionId,
    pub channel: PaymentChannel,
    pub sender_account: String,
    pub receiver_account: String,
    pub amount: Decimal,
    pub currency: Currency,
    pub status: PaymentStatus,
    pub routing_number: Option<String>,
    pub swift_code: Option<String>,
    pub reference: String,
    pub initiated_at: DateTime<Utc>,
    pub settled_at: Option<DateTime<Utc>>,
}

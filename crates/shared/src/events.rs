use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::errors::DomainError;

/// Marker trait for domain events.
pub trait DomainEvent: Send + Sync {
    fn event_type(&self) -> &str;
    fn aggregate_id(&self) -> Uuid;
    fn occurred_at(&self) -> DateTime<Utc>;
}

/// Envelope wrapping a serialised domain event for storage/publishing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub aggregate_type: String,
    pub event_type: String,
    pub payload: Value,
    pub occurred_at: DateTime<Utc>,
    pub sequence_number: i64,
    pub correlation_id: Option<Uuid>,
}

impl EventEnvelope {
    pub fn new(
        aggregate_id: Uuid,
        aggregate_type: impl Into<String>,
        event_type: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            aggregate_id,
            aggregate_type: aggregate_type.into(),
            event_type: event_type.into(),
            payload,
            occurred_at: Utc::now(),
            sequence_number: 0,
            correlation_id: None,
        }
    }
}

/// Async trait for persisting and loading domain events.
#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, envelope: EventEnvelope) -> Result<(), DomainError>;
    async fn load_events(&self, aggregate_id: Uuid) -> Result<Vec<EventEnvelope>, DomainError>;
    async fn load_snapshot(&self, aggregate_id: Uuid) -> Result<Option<EventEnvelope>, DomainError>;
}

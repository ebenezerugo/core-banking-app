use async_trait::async_trait;
use shared::{
    errors::DomainError,
    events::{EventEnvelope, EventStore},
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventStore for PostgresEventStore {
    async fn append(&self, event: EventEnvelope) -> Result<(), DomainError> {
        let seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence_number), 0) FROM event_store WHERE aggregate_id = $1",
        )
        .bind(event.aggregate_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        sqlx::query(
            r#"INSERT INTO event_store (id, aggregate_id, aggregate_type, event_type, payload, sequence_number, correlation_id, occurred_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(event.id)
        .bind(event.aggregate_id)
        .bind(&event.aggregate_type)
        .bind(&event.event_type)
        .bind(&event.payload)
        .bind(seq + 1)
        .bind(event.correlation_id)
        .bind(event.occurred_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        Ok(())
    }

    async fn load_events(&self, aggregate_id: Uuid) -> Result<Vec<EventEnvelope>, DomainError> {
        let rows = sqlx::query(
            r#"SELECT id, aggregate_id, aggregate_type, event_type, payload, sequence_number, correlation_id, occurred_at
               FROM event_store WHERE aggregate_id = $1 ORDER BY sequence_number ASC"#,
        )
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        let envelopes = rows
            .iter()
            .map(|r| EventEnvelope {
                id: r.get("id"),
                aggregate_id: r.get("aggregate_id"),
                aggregate_type: r.get("aggregate_type"),
                event_type: r.get("event_type"),
                payload: r.get("payload"),
                sequence_number: r.get("sequence_number"),
                correlation_id: r.get("correlation_id"),
                occurred_at: r.get("occurred_at"),
            })
            .collect();

        Ok(envelopes)
    }

    async fn load_snapshot(&self, aggregate_id: Uuid) -> Result<Option<EventEnvelope>, DomainError> {
        let row = sqlx::query(
            r#"SELECT id, aggregate_id, aggregate_type, event_type, payload, sequence_number, correlation_id, occurred_at
               FROM event_store WHERE aggregate_id = $1 AND event_type = 'Snapshot'
               ORDER BY sequence_number DESC LIMIT 1"#,
        )
        .bind(aggregate_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        Ok(row.map(|r| EventEnvelope {
            id: r.get("id"),
            aggregate_id: r.get("aggregate_id"),
            aggregate_type: r.get("aggregate_type"),
            event_type: r.get("event_type"),
            payload: r.get("payload"),
            sequence_number: r.get("sequence_number"),
            correlation_id: r.get("correlation_id"),
            occurred_at: r.get("occurred_at"),
        }))
    }
}

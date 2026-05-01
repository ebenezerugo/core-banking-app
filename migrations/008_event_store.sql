-- Event store for event sourcing
CREATE TABLE domain_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_id UUID NOT NULL,
    aggregate_type TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    metadata JSONB DEFAULT '{}',
    sequence_number BIGINT NOT NULL,
    correlation_id UUID,
    causation_id UUID,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT domain_events_sequence_unique UNIQUE (aggregate_id, sequence_number)
);

CREATE INDEX idx_events_aggregate ON domain_events(aggregate_id);
CREATE INDEX idx_events_aggregate_type ON domain_events(aggregate_type);
CREATE INDEX idx_events_event_type ON domain_events(event_type);
CREATE INDEX idx_events_occurred_at ON domain_events(occurred_at);
CREATE INDEX idx_events_correlation ON domain_events(correlation_id);

-- Snapshots for efficient aggregate reconstruction
CREATE TABLE event_snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_id UUID NOT NULL,
    aggregate_type TEXT NOT NULL,
    snapshot_data JSONB NOT NULL,
    sequence_number BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT snapshots_aggregate_unique UNIQUE (aggregate_id)
);

CREATE INDEX idx_snapshots_aggregate ON event_snapshots(aggregate_id);

-- Idempotency store
CREATE TABLE idempotency_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    key TEXT NOT NULL,
    response_status INTEGER NOT NULL,
    response_body JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '24 hours',
    CONSTRAINT idempotency_tenant_key_unique UNIQUE (tenant_id, key)
);

CREATE INDEX idx_idempotency_tenant_key ON idempotency_keys(tenant_id, key);

-- Charge definitions
CREATE TABLE charge_definitions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    charge_type charge_type NOT NULL,
    applied_to TEXT NOT NULL,
    amount NUMERIC(20,6) NOT NULL DEFAULT 0,
    percentage NUMERIC(10,6) NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'USD',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

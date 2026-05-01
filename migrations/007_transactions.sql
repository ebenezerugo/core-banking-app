CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    idempotency_key TEXT NOT NULL,
    transaction_type transaction_type NOT NULL,
    from_account_id UUID REFERENCES accounts(id),
    to_account_id UUID REFERENCES accounts(id),
    amount NUMERIC(20,6) NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    fee NUMERIC(20,6) NOT NULL DEFAULT 0,
    description TEXT NOT NULL DEFAULT '',
    reference TEXT NOT NULL,
    status transaction_status NOT NULL DEFAULT 'pending',
    reversal_of UUID REFERENCES transactions(id),
    correlation_id UUID NOT NULL DEFAULT uuid_generate_v4(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}',
    CONSTRAINT transactions_idempotency_tenant_unique UNIQUE (idempotency_key, tenant_id),
    CONSTRAINT transactions_positive_amount CHECK (amount > 0),
    CONSTRAINT transactions_non_negative_fee CHECK (fee >= 0)
);
CREATE INDEX idx_transactions_tenant_id ON transactions(tenant_id);
CREATE INDEX idx_transactions_from_account ON transactions(from_account_id);
CREATE INDEX idx_transactions_to_account ON transactions(to_account_id);
CREATE INDEX idx_transactions_idempotency ON transactions(idempotency_key, tenant_id);
CREATE INDEX idx_transactions_status ON transactions(status);
CREATE INDEX idx_transactions_correlation ON transactions(correlation_id);

CREATE TABLE applied_charges (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    charge_definition_id UUID NOT NULL,
    account_id UUID REFERENCES accounts(id),
    loan_id UUID REFERENCES loans(id),
    transaction_id UUID REFERENCES transactions(id),
    amount NUMERIC(20,6) NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    waived BOOLEAN NOT NULL DEFAULT false,
    waiver_reason TEXT,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

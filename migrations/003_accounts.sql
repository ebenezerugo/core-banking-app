CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    account_number TEXT NOT NULL,
    customer_id UUID NOT NULL REFERENCES customers(id),
    account_type account_type NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    balance NUMERIC(20,6) NOT NULL DEFAULT 0,
    available_balance NUMERIC(20,6) NOT NULL DEFAULT 0,
    status account_status NOT NULL DEFAULT 'active',
    interest_rate NUMERIC(10,6),
    maturity_date TIMESTAMPTZ,
    minimum_balance NUMERIC(20,6) NOT NULL DEFAULT 0,
    overdraft_limit NUMERIC(20,6) NOT NULL DEFAULT 0,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT accounts_account_number_tenant_unique UNIQUE (account_number, tenant_id)
);
CREATE INDEX idx_accounts_tenant_id ON accounts(tenant_id);
CREATE INDEX idx_accounts_customer_id ON accounts(customer_id);
CREATE INDEX idx_accounts_account_number ON accounts(account_number);
CREATE INDEX idx_accounts_status ON accounts(status);

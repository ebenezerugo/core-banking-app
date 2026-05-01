CREATE TABLE loans (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL,
    customer_id UUID NOT NULL REFERENCES customers(id),
    account_id UUID NOT NULL REFERENCES accounts(id),
    principal NUMERIC(20,6) NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    interest_rate NUMERIC(10,6) NOT NULL,
    interest_type interest_type NOT NULL DEFAULT 'simple',
    term_months INTEGER NOT NULL,
    repayment_schedule_type repayment_schedule_type NOT NULL DEFAULT 'annuity',
    disbursement_date TIMESTAMPTZ,
    first_repayment_date DATE,
    maturity_date DATE,
    outstanding_principal NUMERIC(20,6) NOT NULL DEFAULT 0,
    outstanding_interest NUMERIC(20,6) NOT NULL DEFAULT 0,
    status loan_status NOT NULL DEFAULT 'draft',
    arrears_days INTEGER NOT NULL DEFAULT 0,
    approved_by UUID,
    approved_at TIMESTAMPTZ,
    disbursed_by UUID,
    notes TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE loan_repayment_schedules (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    loan_id UUID NOT NULL REFERENCES loans(id),
    installment_number INTEGER NOT NULL,
    due_date DATE NOT NULL,
    principal_due NUMERIC(20,6) NOT NULL,
    interest_due NUMERIC(20,6) NOT NULL,
    total_due NUMERIC(20,6) NOT NULL,
    principal_paid NUMERIC(20,6) NOT NULL DEFAULT 0,
    interest_paid NUMERIC(20,6) NOT NULL DEFAULT 0,
    status installment_status NOT NULL DEFAULT 'pending',
    paid_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loans_tenant_id ON loans(tenant_id);
CREATE INDEX idx_loans_customer_id ON loans(customer_id);
CREATE INDEX idx_loans_status ON loans(status);
CREATE INDEX idx_loan_schedules_loan_id ON loan_repayment_schedules(loan_id);
CREATE INDEX idx_loan_schedules_due_date ON loan_repayment_schedules(due_date);

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Custom types
CREATE TYPE customer_type AS ENUM ('individual', 'business', 'group');
CREATE TYPE customer_status AS ENUM ('pending', 'active', 'suspended', 'closed');
CREATE TYPE kyc_status AS ENUM ('not_started', 'in_progress', 'verified', 'rejected', 'expired');
CREATE TYPE account_type AS ENUM ('savings', 'current', 'fixed_deposit', 'recurring_deposit', 'loan');
CREATE TYPE account_status AS ENUM ('active', 'dormant', 'frozen', 'closed');
CREATE TYPE loan_status AS ENUM ('draft', 'submitted', 'under_review', 'approved', 'disbursed', 'active', 'arrears', 'write_off', 'closed', 'rejected');
CREATE TYPE transaction_status AS ENUM ('pending', 'processing', 'completed', 'failed', 'reversed');
CREATE TYPE transaction_type AS ENUM ('deposit', 'withdrawal', 'transfer', 'loan_disbursement', 'loan_repayment', 'fee_charge', 'interest_application', 'reversal');
CREATE TYPE payment_channel AS ENUM ('ach', 'swift', 'internal_transfer', 'mobile_money', 'card');
CREATE TYPE payment_status AS ENUM ('initiated', 'processing', 'settled', 'failed', 'rejected');
CREATE TYPE journal_status AS ENUM ('draft', 'posted', 'reversed');
CREATE TYPE account_class AS ENUM ('asset', 'liability', 'equity', 'income', 'expense');
CREATE TYPE debit_credit AS ENUM ('debit', 'credit');
CREATE TYPE installment_status AS ENUM ('pending', 'partially_paid', 'paid', 'overdue');
CREATE TYPE charge_type AS ENUM ('flat', 'percentage', 'tiered');
CREATE TYPE repayment_schedule_type AS ENUM ('flat', 'declining_balance', 'annuity');
CREATE TYPE interest_type AS ENUM ('simple', 'compound');

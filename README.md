# Core Banking App

A production-ready core banking platform built in Rust — a high-performance replacement for Apache Fineract, eliminating JVM overhead, memory bloat, and operational complexity.

## Architecture

The system combines three complementary architectural patterns:

- **Microservices** — separate crates per domain (customers, loans, accounts, accounting, payments, transactions, charges)
- **Event Sourcing + CQRS** — immutable domain event ledger for all state changes; full audit trail; state reconstruction via event replay
- **Hexagonal / Ports-and-Adapters** — domain core isolated from infrastructure; repository traits abstract over PostgreSQL

```
┌───────────────────────────────────────────────────────────────────┐
│                        api-gateway (Axum)                         │
│  JWT auth │ idempotency │ structured tracing │ OpenAPI/Swagger    │
└───────┬───────┬───────┬───────┬───────┬──────┬──────────────────┘
        │       │       │       │       │      │
   customers  loans  accounts accounting payments transactions
        │       │       │       │       │      │
        └───────┴───────┴───────┴───────┴──────┘
                         │
              PostgreSQL (sqlx, compile-time queries)
                         │
              NATS JetStream (domain event bus)
```

## Workspace Layout

```
crates/
├── shared/          # Money<C>, Currency, ID newtypes, DomainError, EventStore trait, JWT/RBAC
├── domain/          # Pure domain models — no I/O, no framework deps
├── customers/       # Customer & KYC management (CRUD, onboarding, KYC workflow)
├── loans/           # Loan lifecycle (origination → disburse → repay → close/write-off)
├── accounts/        # Deposit & savings accounts + compound/simple interest engine
├── accounting/      # Double-entry GL (enforces Σ debits == Σ credits at type level)
├── payments/        # ACH, SWIFT, internal transfers, mobile money
├── transactions/    # Atomic multi-leg transaction engine with idempotency + reversal
├── charges/         # Configurable fee/penalty structures with waiver workflows
└── api-gateway/     # Axum HTTP server, middleware, router, event store (PostgreSQL)
```

## Key Design Decisions

| Concern | Choice | Reason |
|---------|--------|--------|
| Monetary arithmetic | `rust_decimal::Decimal` | Zero floating-point error |
| DB access | `sqlx` runtime queries | No `DATABASE_URL` at compile time; works in CI |
| State invariants | Newtype wrappers (`Money<C>`, `AccountId`, …) | Invalid states unrepresentable |
| Double-entry | Validated at domain model layer | `JournalEntry::validate()` enforces Σ debit == Σ credit |
| Idempotency | `idempotency_keys` table + request middleware | Safe retry semantics for all mutation endpoints |
| Audit trail | `domain_events` event store table | Replay any aggregate to any point in time |
| Auth | JWT + RBAC (`Role` enum) | Tenant-scoped, role-enforced access |
| Async | Tokio + Axum | Non-blocking I/O throughout |

## Modules

### Customer & KYC Management
- Individual, business, and group accounts
- KYC status workflow: `NotStarted → InProgress → Verified / Rejected`
- PII fields never logged (structured tracing excludes them)

### Loan Lifecycle Management
- Full origination → underwriting → approval → disbursement → repayment → close / write-off flow
- Three amortization algorithms: **Annuity (EMI)**, **Declining Balance**, **Flat Rate**
- Arrears tracking, restructuring, collections

### Savings & Deposit Accounts
- Account types: Savings, Current, Fixed Deposit, Recurring Deposit, Loan
- Simple and compound interest calculation engines
- Optimistic locking via `version` column

### Accounting (Double-Entry GL)
- Chart of accounts (hierarchical)
- Journal entries with debit/credit lines; invariant enforced: `Σ debits == Σ credits`
- Trial balance, financial statements

### Payments & Transfers
- Channels: ACH, SWIFT, Internal Transfer, Mobile Money, Card
- Full settlement lifecycle

### Transaction Engine
- Atomic multi-leg transfers (single DB transaction)
- Idempotency key deduplication
- Reversal support with full audit trail via event sourcing

### Charges & Fees
- Fee types: Flat, Percentage, Tiered
- Configurable per product
- Waiver workflow

## API Endpoints

| Group | Path Prefix | Description |
|-------|------------|-------------|
| Health | `GET /health` | Liveness probe |
| Customers | `/api/v1/customers` | CRUD + KYC update |
| Accounts | `/api/v1/accounts` | Open, deposit, withdraw, statement |
| Loans | `/api/v1/loans` | Apply, approve, disburse, repay, schedule |
| Transactions | `/api/v1/transactions` | Execute, reverse, history |
| Payments | `/api/v1/payments` | Initiate, status |
| Accounting | `/api/v1/accounting` | Journal entries, CoA, trial balance |
| Charges | `/api/v1/charges` | Definitions, apply, waive |
| Swagger UI | `/swagger-ui` | Interactive API docs |

All mutation endpoints require `Idempotency-Key` header and `Authorization: Bearer <JWT>`.

## Database Schema

8 ordered migrations in `migrations/`:

| File | Contents |
|------|----------|
| `001_initial_schema.sql` | PostgreSQL enum types |
| `002_customers.sql` | `customers` table |
| `003_accounts.sql` | `accounts` table |
| `004_loans.sql` | `loans` + `loan_repayment_schedules` |
| `005_accounting.sql` | `chart_of_accounts`, `journal_entries`, `journal_lines` |
| `006_payments.sql` | `payments` table |
| `007_transactions.sql` | `transactions`, `applied_charges` |
| `008_event_store.sql` | `domain_events`, `event_snapshots`, `idempotency_keys`, `charge_definitions` |

## Getting Started

### Prerequisites
- Rust (stable, edition 2021)
- Docker & Docker Compose

### Run with Docker Compose

```bash
git clone https://github.com/ebenezerugo/core-banking-app
cd core-banking-app

# Copy and edit the environment file
cp .env.example .env
# Edit .env and set JWT_SECRET to a secure random value

docker compose up --build
```

The API will be available at `http://localhost:8080`.
Swagger UI: `http://localhost:8080/swagger-ui`

### Run Locally

```bash
# Start PostgreSQL and NATS
docker compose up db nats -d

export DATABASE_URL=postgresql://banking:banking_secret@localhost:5432/corebanking
export JWT_SECRET=dev_secret_change_me
export PORT=8080
export RUST_LOG=info

cargo run -p api-gateway
```

### Run Tests

```bash
cargo test
```

Includes:
- Unit tests for domain logic (account debit/credit, double-entry journal validation)
- Property-based tests (`proptest`) for the amortization engine — verifies total principal repaid == loan amount across thousands of random inputs
- Interest calculation tests

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | — | PostgreSQL connection string (required) |
| `PORT` | `8080` | HTTP listen port |
| `JWT_SECRET` | — | HS256 signing secret (required, min 256 bits) |
| `NATS_URL` | `nats://localhost:4222` | NATS server for domain events |
| `TENANT_ID` | `default` | Default tenant identifier |
| `RUST_LOG` | `info` | Log level filter |

## Security Notes

- JWT secrets must be rotated regularly and never committed to source control
- All PII fields are excluded from structured log output
- Use row-level security in PostgreSQL for multi-tenant isolation in production
- Idempotency keys expire after 24 hours

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust (stable, edition 2021) |
| Async runtime | Tokio |
| Web framework | Axum 0.7 |
| Database | PostgreSQL 16 via sqlx 0.7 |
| Migrations | sqlx-migrate (embedded) |
| Serialization | serde + serde_json |
| Auth | JWT (jsonwebtoken) + RBAC |
| Messaging | NATS JetStream |
| Money | rust_decimal (fixed-point, no floats) |
| Errors | thiserror (domain) + anyhow (application) |
| Logging | tracing + tracing-subscriber (JSON) |
| Config | environment variables |
| API docs | utoipa (OpenAPI 3) + Swagger UI |
| Testing | proptest (property-based) |
| Containers | Docker (multi-stage) + Docker Compose |

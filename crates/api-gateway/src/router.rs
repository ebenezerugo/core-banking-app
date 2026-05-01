use std::sync::Arc;

use axum::{middleware, routing, Router};
use sqlx::PgPool;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use accounts::{repository::PostgresAccountRepository, service::AccountService};
use accounting::{repository::PostgresAccountingRepository, service::AccountingService};
use charges::{repository::PostgresChargeRepository, service::ChargeService};
use customers::{repository::PostgresCustomerRepository, service::CustomerService};
use loans::{repository::PostgresLoanRepository, service::LoanService};
use payments::{repository::PostgresPaymentRepository, service::PaymentService};
use transactions::{engine::TransactionEngine, repository::PostgresTransactionRepository};

use crate::config::AppConfig;
use crate::event_store::PostgresEventStore;
use crate::middleware::auth::auth_middleware;
use crate::middleware::tracing::tracing_middleware;

pub fn build_router(pool: PgPool, config: AppConfig) -> Router {
    let event_store = Arc::new(PostgresEventStore::new(pool.clone()));

    let customer_svc = Arc::new(CustomerService::new(
        PostgresCustomerRepository::new(pool.clone()),
        event_store.clone(),
    ));
    let account_svc = Arc::new(AccountService::new(
        PostgresAccountRepository::new(pool.clone()),
        event_store.clone(),
    ));
    let accounting_svc = Arc::new(AccountingService::new(
        PostgresAccountingRepository::new(pool.clone()),
    ));
    let loan_svc = Arc::new(LoanService::new(
        PostgresLoanRepository::new(pool.clone()),
        event_store.clone(),
    ));
    let payment_svc = Arc::new(PaymentService::new(
        PostgresPaymentRepository::new(pool.clone()),
    ));
    let charge_svc = Arc::new(ChargeService::new(
        PostgresChargeRepository::new(pool.clone()),
    ));
    let tx_repo = Arc::new(PostgresTransactionRepository::new(pool.clone()));
    let tx_engine = Arc::new(TransactionEngine::new(
        Arc::new(pool.clone()),
        tx_repo,
        event_store.clone(),
    ));

    // Build each service sub-router independently to avoid `with_state` type conflicts.
    let customers_router = Router::new()
        .route("/customers", routing::post(customers::handlers::create_customer))
        .route("/customers/:tenant_id/:customer_id", routing::get(customers::handlers::get_customer))
        .route("/customers/:tenant_id/:customer_id/kyc", routing::put(customers::handlers::update_kyc))
        .with_state(customer_svc);

    let accounts_router = Router::new()
        .route("/accounts", routing::post(accounts::handlers::open_account))
        .route("/accounts/:tenant_id/:account_id", routing::get(accounts::handlers::get_account))
        .route("/accounts/:tenant_id/:account_id/deposit", routing::post(accounts::handlers::deposit))
        .route("/accounts/:tenant_id/:account_id/withdraw", routing::post(accounts::handlers::withdraw))
        .with_state(account_svc);

    let loans_router = Router::new()
        .route("/loans", routing::post(loans::handlers::create_loan))
        .route("/loans/:tenant_id/:loan_id", routing::get(loans::handlers::get_loan))
        .route("/loans/:tenant_id/:loan_id/approve", routing::post(loans::handlers::approve_loan))
        .route("/loans/:tenant_id/:loan_id/disburse", routing::post(loans::handlers::disburse_loan))
        .route("/loans/:tenant_id/:loan_id/repay", routing::post(loans::handlers::repay_loan))
        .with_state(loan_svc);

    let accounting_router = Router::new()
        .route("/accounting/journal", routing::post(accounting::handlers::post_journal_entry))
        .route("/accounting/trial-balance", routing::get(accounting::handlers::get_trial_balance))
        .with_state(accounting_svc);

    let payments_router = Router::new()
        .route("/payments", routing::post(payments::handlers::initiate_payment))
        .route("/payments/:tenant_id/:payment_id", routing::get(payments::handlers::get_payment))
        .route("/payments/:tenant_id/:payment_id/settle", routing::post(payments::handlers::settle_payment))
        .with_state(payment_svc);

    let transactions_router = Router::new()
        .route("/transactions", routing::post(transactions::handlers::execute_transaction))
        .route("/transactions/:tenant_id/:tx_id/reverse", routing::post(transactions::handlers::reverse_transaction))
        .with_state(tx_engine);

    let charges_router = Router::new()
        .route("/charges/:tenant_id/:definition_id/apply", routing::post(charges::handlers::apply_charge))
        .route("/charges/:charge_id/waive", routing::post(charges::handlers::waive_charge))
        .with_state(charge_svc);

    let api = Router::new()
        .merge(customers_router)
        .merge(accounts_router)
        .merge(loans_router)
        .merge(accounting_router)
        .merge(payments_router)
        .merge(transactions_router)
        .merge(charges_router);

    let jwt_secret = config.jwt_secret.clone();

    Router::new()
        .nest("/api/v1", api)
        .layer(middleware::from_fn_with_state(jwt_secret, auth_middleware))
        .layer(middleware::from_fn(tracing_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

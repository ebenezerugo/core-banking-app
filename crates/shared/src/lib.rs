pub mod auth;
pub mod errors;
pub mod events;
pub mod pagination;
pub mod types;

pub use auth::{AuthContext, Claims, Role};
pub use errors::{DomainError, InfrastructureError};
pub use events::{DomainEvent, EventEnvelope, EventStore};
pub use pagination::{PageRequest, PageResponse};
pub use types::{
    AccountId, AccountNumber, Currency, CustomerId, JournalEntryId, LoanId, Money, TenantId,
    TransactionId,
};

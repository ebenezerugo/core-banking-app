use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    SuperAdmin,
    Admin,
    LoanOfficer,
    Teller,
    Auditor,
    Customer,
}

/// JWT claims stored inside a token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject – typically the user UUID.
    pub sub: String,
    pub role: Role,
    pub tenant_id: Uuid,
    /// Expiry as UNIX timestamp.
    pub exp: usize,
}

/// Resolved auth context attached to every authenticated request.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub role: Role,
    pub tenant_id: Uuid,
}

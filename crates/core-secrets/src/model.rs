//! Public metadata; secret material has no printable representation.

#[derive(Clone)]
pub struct SecretKey(pub(super) [u8; 32]);

impl SecretKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey(..)")
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct EnvEntry {
    pub name: String,
    pub updated_at: String,
}

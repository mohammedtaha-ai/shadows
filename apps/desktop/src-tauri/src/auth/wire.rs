//! Backend-owned auth-v1 1.1.0 DTO consumer. No identity policy lives here.
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Principal {
    pub account_id: String,
    pub device_id: String,
    pub session_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginStart {
    pub attempt_id: String,
    pub authorization_url: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionGrant {
    pub handle: String,
    pub principal: Principal,
    pub expires_at: String,
    pub enrollment_credential: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogoutResult {
    pub local: String,
    pub provider: String,
    pub browser_logout_url: Option<String>,
}
pub fn opaque(value: &str) -> bool {
    value.len() == 43
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
pub fn ticket(url: &url::Url) -> Result<String, &'static str> {
    let pairs: Vec<_> = url.query_pairs().collect();
    if url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || url.path() != "/oidc-return"
        || url.fragment().is_some()
        || pairs.len() != 1
        || pairs[0].0 != "ticket"
        || !opaque(&pairs[0].1)
    {
        return Err("return_invalid");
    }
    Ok(pairs[0].1.to_string())
}
pub fn checked_url(value: &str, base: &url::Url, path: &str) -> Result<url::Url, &'static str> {
    let url = url::Url::parse(value).map_err(|_| "url_invalid")?;
    if url.scheme() != "https"
        || url.origin() != base.origin()
        || url.path() != path
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("url_invalid");
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_requires_exactly_one_opaque_ticket() {
        let value = "a".repeat(43);
        let u = url::Url::parse(&format!(
            "http://127.0.0.1:34567/oidc-return?ticket={value}"
        ))
        .unwrap();
        assert_eq!(ticket(&u).unwrap(), value);
        for query in [
            "ticket=x",
            "ticket=x&ticket=y",
            "ticket=x&handle=y",
            "error=denied",
        ] {
            assert!(ticket(
                &url::Url::parse(&format!("http://127.0.0.1:34567/oidc-return?{query}")).unwrap()
            )
            .is_err());
        }
    }
    #[test]
    fn browser_urls_require_reviewed_origin_and_operation_path() {
        let base = url::Url::parse("https://127.0.0.1:8443").unwrap();
        assert!(checked_url(
            "https://127.0.0.1:8443/realms/shadows-local/protocol/openid-connect/auth?state=x",
            &base,
            "/realms/shadows-local/protocol/openid-connect/auth"
        )
        .is_ok());
        for v in [
            "http://127.0.0.1:8443/ok",
            "https://evil.example/ok",
            "https://u:p@127.0.0.1:8443/ok",
            "https://127.0.0.1:8443/ok#x",
            "https://127.0.0.1:8443/other",
        ] {
            assert!(checked_url(v, &base, "/ok").is_err());
        }
    }
    #[test]
    fn grant_rejects_extra_fields_and_projection_excludes_credentials() {
        let json = r#"{"handle":"x","principal":{"accountId":"a","deviceId":"b","sessionId":"c"},"expiresAt":"date","providerToken":"forbidden"}"#;
        assert!(serde_json::from_str::<SessionGrant>(json).is_err());
        assert!(opaque(&"a".repeat(43)));
        assert!(!opaque(&"a".repeat(42)));
    }
    #[test]
    fn pinned_backend_fixtures_deserialize_without_provider_tokens() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/auth-v1.1.json")).unwrap();
        assert_eq!(fixtures["contractVersion"], "1.1.0");
        let grant: SessionGrant =
            serde_json::from_value(fixtures["newEnrollmentGrant"].clone()).unwrap();
        assert!(opaque(&grant.handle));
        assert!(opaque(grant.enrollment_credential.as_deref().unwrap()));
        let result: LogoutResult =
            serde_json::from_value(fixtures["providerPending"].clone()).unwrap();
        assert_eq!(result.local, "revoked");
        assert_eq!(result.provider, "pending");
    }
}

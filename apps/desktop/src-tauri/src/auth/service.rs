use super::{
    http::Backend,
    store::{Credentials, Store},
    wire,
};
use crate::native_login::{self, ReturnBinding};
use reqwest::Method;
use serde::Serialize;
use serde_json::json;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub state: &'static str,
    pub has_enrollment: bool,
    pub principal: Option<wire::Principal>,
    pub provider_logout: Option<&'static str>,
}
pub struct AuthService {
    backend: Backend,
    store: Store,
}
impl AuthService {
    pub fn assistant(
        &self,
        request: crate::assistant::Request,
        expected_account: &str,
        expected_session: &str,
    ) -> Result<serde_json::Value, &'static str> {
        let operation = request.operation()?;
        let credentials = self.store.load()?;
        let handle = credentials
            .handle
            .as_deref()
            .ok_or("verification_refused")?;
        // A queued IPC worker may start after login changed. Resolve the public
        // scope using this exact protected handle before dispatching any write.
        let principal = self.backend.request::<wire::Principal>(
            Method::GET,
            "/v1/auth/session",
            None,
            Some(handle),
        )?;
        if !valid_principal(&principal)
            || principal.account_id != expected_account
            || principal.session_id != expected_session
            || credentials.account.as_deref() != Some(expected_account)
            || self.store.load()?.handle.as_deref() != Some(handle)
        {
            return Err("source_changed");
        }
        let result = self.backend.request_bounded(
            operation.method,
            &operation.path,
            operation.body,
            Some(handle),
            131072,
        )?;
        if self.store.load()?.handle.as_deref() != Some(handle) {
            return Err("verification_refused");
        }
        Ok(result)
    }
    pub fn from_environment(path: PathBuf) -> Result<Self, &'static str> {
        let backend = Backend::from_environment()?;
        let store = Store::new(path, backend.base.origin().ascii_serialization());
        Ok(Self { backend, store })
    }
    pub fn status(&self) -> Result<View, &'static str> {
        let c = self.store.load()?;
        let Some(handle) = &c.handle else {
            return Ok(Self::view("signed_out", &c, None));
        };
        let result = self.backend.request::<wire::Principal>(
            Method::GET,
            "/v1/auth/session",
            None,
            Some(handle),
        );
        match result {
            Ok(p) if valid_principal(&p) && c.account.as_deref() == Some(&p.account_id) => {
                Ok(Self::view("signed_in", &c, Some(p)))
            }
            Ok(_) => Err("response_invalid"),
            Err("verification_refused") => Ok(Self::view("verification_refused", &c, None)),
            Err("backend_unavailable") => Ok(Self::view("unavailable", &c, None)),
            Err(e) => Err(e),
        }
    }
    fn view(state: &'static str, c: &Credentials, principal: Option<wire::Principal>) -> View {
        View {
            state,
            has_enrollment: c.enrollment.is_some(),
            principal,
            provider_logout: None,
        }
    }
    pub fn login(
        &self,
        new_enrollment: bool,
        cancelled: &AtomicBool,
    ) -> Result<View, &'static str> {
        self.login_with_browser(new_enrollment, cancelled, |u, b| {
            native_login::open_system_browser(u, b).map_err(|_| "browser_unavailable")
        })
    }
    pub(super) fn login_with_browser(
        &self,
        new_enrollment: bool,
        cancelled: &AtomicBool,
        open: impl FnOnce(&url::Url, &url::Url) -> Result<(), &'static str>,
    ) -> Result<View, &'static str> {
        let prior = self.store.load()?;
        if !new_enrollment && prior.enrollment.is_none() {
            return Err("enrollment_required");
        }
        let binding = ReturnBinding::new().map_err(|_| "listener_unavailable")?;
        let started: wire::LoginStart = self.backend.request(Method::POST, "/v1/auth/login", Some(json!({
            "platform":"windows", "returnUri":binding.return_uri(), "returnChallenge":binding.challenge()
        })), None)?;
        if !uuid(&started.attempt_id) {
            return Err("response_invalid");
        }
        let browser = wire::checked_url(
            &started.authorization_url,
            &self.backend.base,
            "/realms/shadows-local/protocol/openid-connect/auth",
        )?;
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled");
        }
        open(&browser, &self.backend.base)?;
        let returned = binding
            .wait_cancellable(Duration::from_secs(180), cancelled)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::Interrupted => "cancelled",
                std::io::ErrorKind::TimedOut => "login_timeout",
                _ => "return_invalid",
            })?;
        let ticket = wire::ticket(&returned.callback)?;
        let enrollment = if new_enrollment {
            json!({"mode":"new"})
        } else {
            json!({"mode":"reuse", "credential":prior.enrollment.as_ref().ok_or("enrollment_required")?})
        };
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled");
        }
        let grant: wire::SessionGrant = self.backend.request(
            Method::POST,
            "/v1/auth/return",
            Some(json!({
                "ticket":ticket, "returnVerifier":returned.verifier(), "enrollment":enrollment
            })),
            None,
        )?;
        if !wire::opaque(&grant.handle)
            || !valid_principal(&grant.principal)
            || grant.expires_at.is_empty()
            || (new_enrollment
                && !grant
                    .enrollment_credential
                    .as_deref()
                    .is_some_and(wire::opaque))
            || (!new_enrollment
                && (grant.enrollment_credential.is_some()
                    || prior.account.as_deref() != Some(&grant.principal.account_id)))
        {
            return Err("response_invalid");
        }
        let next = Credentials {
            handle: Some(grant.handle),
            enrollment: if new_enrollment {
                grant.enrollment_credential
            } else {
                prior.enrollment
            },
            account: Some(grant.principal.account_id.clone()),
            device: Some(grant.principal.device_id.clone()),
            origin: prior.origin,
        };
        if cancelled.load(Ordering::Relaxed) {
            let _ = self.backend.request::<wire::LogoutResult>(
                Method::POST,
                "/v1/auth/logout",
                Some(json!({})),
                next.handle.as_deref(),
            );
            return Err("cancelled");
        }
        if let Err(e) = self.store.save(&next) {
            let _ = self.backend.request::<wire::LogoutResult>(
                Method::POST,
                "/v1/auth/logout",
                Some(json!({})),
                next.handle.as_deref(),
            );
            return Err(e);
        }
        self.status()
    }
    pub fn logout(&self, all: bool) -> Result<View, &'static str> {
        self.logout_with_browser(all, |u, b| {
            native_login::open_system_browser(u, b).map_err(|_| "browser_unavailable")
        })
    }
    pub(super) fn logout_with_browser(
        &self,
        all: bool,
        open: impl FnOnce(&url::Url, &url::Url) -> Result<(), &'static str>,
    ) -> Result<View, &'static str> {
        let mut c = self.store.load()?;
        let handle = c.handle.as_deref().ok_or("session_required")?;
        let result = self.backend.request::<wire::LogoutResult>(
            Method::POST,
            if all {
                "/v1/auth/logout-all"
            } else {
                "/v1/auth/logout"
            },
            Some(json!({})),
            Some(handle),
        );
        match result {
            Err(e) => Err(e),
            Ok(result) => {
                if result.local != "revoked"
                    || !["pending", "complete"].contains(&result.provider.as_str())
                {
                    return Err("response_invalid");
                }
                c.handle = None;
                self.store.save(&c)?;
                let mut view = Self::view("signed_out", &c, None);
                view.provider_logout = Some(if result.provider == "complete" {
                    "complete"
                } else {
                    "pending"
                });
                if let Some(value) = result.browser_logout_url {
                    let url =
                        wire::checked_url(&value, &self.backend.base, "/v1/auth/provider-logout")?;
                    open(&url, &self.backend.base)?;
                }
                Ok(view)
            }
        }
    }
    pub fn revoke_current_device(&self) -> Result<View, &'static str> {
        let mut c = self.store.load()?;
        let handle = c.handle.as_deref().ok_or("session_required")?;
        // A stored device id is only a revocation target; the backend still authorizes it.
        let device = c
            .device
            .as_deref()
            .filter(|v| uuid(v))
            .ok_or("device_required")?;
        let _: serde_json::Value = self.backend.request(
            Method::DELETE,
            &format!("/v1/devices/{device}"),
            None,
            Some(handle),
        )?;
        c.handle = None;
        c.enrollment = None;
        c.account = None;
        c.device = None;
        self.store.save(&c)?;
        Ok(Self::view("device_revoked", &c, None))
    }
}
fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
fn valid_principal(p: &wire::Principal) -> bool {
    uuid(&p.account_id) && uuid(&p.device_id) && uuid(&p.session_id)
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;

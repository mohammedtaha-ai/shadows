#[cfg(test)]
mod acceptance;
mod http;
pub mod service;
mod store;
pub mod wire;

use service::{AuthService, View};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
pub struct AuthController {
    path: PathBuf,
    busy: AtomicBool,
    cancelled: AtomicBool,
}
impl AuthController {
    pub fn assistant(
        &self,
        request: crate::assistant::Request,
        expected_account: &str,
        expected_session: &str,
    ) -> Result<serde_json::Value, &'static str> {
        if self.busy.load(Ordering::SeqCst) {
            return Err("busy");
        }
        AuthService::from_environment(self.path.clone())?.assistant(
            request,
            expected_account,
            expected_session,
        )
    }
    pub fn new(path: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            path,
            busy: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        })
    }
    /// Ask an in-flight login to stop; never waits for the worker.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
    pub fn perform(&self, action: &str) -> Result<View, &'static str> {
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("busy");
        }
        struct Release<'a>(&'a AtomicBool);
        impl Drop for Release<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::SeqCst);
            }
        }
        let _release = Release(&self.busy);
        if action == "login" || action == "enroll" {
            self.cancelled.store(false, Ordering::SeqCst);
        }
        let service = AuthService::from_environment(self.path.clone())?;
        match action {
            "status" => service.status(),
            "login" => service.login(false, &self.cancelled),
            "enroll" => service.login(true, &self.cancelled),
            "logout" => service.logout(false),
            "logout_all" => service.logout(true),
            "revoke_device" => service.revoke_current_device(),
            _ => Err("action_invalid"),
        }
    }
}

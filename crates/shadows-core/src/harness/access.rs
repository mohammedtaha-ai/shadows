//! Session operations exposed by the Harness owner.
use super::{Harness, LeaseError, OpenError, OpenSession};
use crate::CoreError;
use crate::threads::ThreadId;
use shadows_agent::{TurnSettings, acp::AcpError, choices::Offered, events::HarnessEvent};
use std::{io, time::Duration};
use tokio::sync::mpsc;

impl Harness {
    #[cfg(feature = "test-support")]
    pub(crate) fn aylul_owners(&self) -> crate::app::AylulOwners {
        self.sessions.owners.clone()
    }
    pub async fn open(&self, thread: &ThreadId) -> Result<OpenSession, OpenError> {
        self.sessions.open(thread).await
    }
    pub async fn offered(&self, thread: &ThreadId) -> Option<Offered> {
        self.sessions.offered(thread).await
    }
    pub async fn lease_events(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
    ) -> Result<mpsc::UnboundedReceiver<HarnessEvent>, LeaseError> {
        self.sessions.lease_events(thread, opened).await
    }
    pub async fn give_back_events(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        rx: mpsc::UnboundedReceiver<HarnessEvent>,
    ) -> () {
        self.sessions.give_back_events(thread, opened, rx).await
    }
    pub async fn mark_answered(&self, thread: &ThreadId, opened: &OpenSession) -> () {
        self.sessions.mark_answered(thread, opened).await
    }
    pub async fn touch(&self, thread: &ThreadId) -> () {
        self.sessions.touch(thread).await
    }
    pub async fn terminate(&self, thread: &ThreadId) -> io::Result<()> {
        self.sessions.terminate(thread).await
    }
    pub async fn terminate_adapter(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
    ) -> io::Result<()> {
        self.sessions.terminate_adapter(thread, opened).await
    }
    pub async fn close_all(&self) -> io::Result<()> {
        self.sessions.close_all().await
    }
    pub fn adapter(&self) -> &shadows_agent::claude::ClaudeAdapter {
        self.sessions.adapter()
    }
    pub fn cancel_wait(&self) -> Duration {
        self.sessions.cancel_wait()
    }
    pub async fn harness_available(&self, kind: &str) -> Result<bool, CoreError> {
        self.sessions.harness_available(kind).await
    }

    pub async fn set_option(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        config_id: &str,
        value: &str,
    ) -> Result<Offered, AcpError> {
        self.sessions
            .set_option(thread, opened, config_id, value)
            .await
    }
    pub async fn prepare_turn(
        &self,
        thread: &ThreadId,
        opened: &OpenSession,
        settings: &TurnSettings,
    ) -> Result<(), String> {
        self.sessions.prepare_turn(thread, opened, settings).await
    }
    #[cfg(feature = "test-support")]
    pub async fn live_count(&self) -> usize {
        self.sessions.live_count().await
    }
    #[cfg(feature = "test-support")]
    pub async fn pid(&self, thread: &ThreadId) -> Option<u32> {
        self.sessions.pid(thread).await
    }
    #[cfg(feature = "test-support")]
    pub async fn force_termination_failure(&self, thread: &ThreadId) -> bool {
        self.sessions.force_termination_failure(thread).await
    }
    #[cfg(feature = "test-support")]
    pub async fn take_events(
        &self,
        thread: &ThreadId,
    ) -> Option<mpsc::UnboundedReceiver<HarnessEvent>> {
        self.sessions.take_events(thread).await
    }
    pub(crate) async fn context_facts(
        &self,
        thread: &ThreadId,
        invocation: Option<super::InvocationContext>,
        stage: String,
    ) -> Result<super::setup::ContextFacts, CoreError> {
        self.sessions
            .setups
            .context_facts(thread, invocation, stage)
            .await
    }
    pub(crate) fn withhold(&self, thread: &ThreadId, skipped: bool) {
        self.sessions.setups.withhold(thread, skipped);
    }
    pub async fn remembered_settings(
        &self,
        kind: &str,
    ) -> Result<Option<(String, Option<String>)>, core_db::StorageError> {
        self.store.remembered_settings(kind).await
    }
    pub async fn latest_limits(
        &self,
        kind: &str,
    ) -> Result<Option<shadows_agent::events::AccountLimits>, core_db::StorageError> {
        self.store.latest_limits(kind).await
    }
    pub async fn record_limits(
        &self,
        kind: &str,
        limits: &shadows_agent::events::AccountLimits,
    ) -> Result<(), core_db::StorageError> {
        self.store.record_limits(kind, limits).await
    }
    #[cfg(feature = "test-support")]
    pub async fn remember_for_test(&self, kind: &str, model: &str, effort: Option<&str>) {
        self.store.remember_for_test(kind, model, effort).await;
    }
}

#[cfg(feature = "test-support")]
impl Harness {
    pub async fn set_thread_harness_for_test(
        storage: &std::sync::Arc<core_db::Storage>,
        ctx: &core_db::CommandContext,
        thread: &ThreadId,
        harness: &str,
    ) -> Result<crate::threads::PlanningThread, CoreError> {
        Ok(super::store::HarnessStore::with_storage(storage.clone())
            .set_thread_harness(ctx, thread, harness)
            .await?)
    }
}

impl super::Harness {
    pub(crate) fn require_images(
        &self,
        opened: &super::OpenSession,
        has_images: bool,
    ) -> Result<(), crate::CoreError> {
        Self::check_images(opened.connection(), has_images)
    }

    pub(super) fn check_images(
        connection: &shadows_agent::acp::Connection,
        has_images: bool,
    ) -> Result<(), crate::CoreError> {
        let caps = connection.prompt_capabilities();
        if has_images && !caps.image {
            return Err(crate::CoreError::Refused {
                code: crate::ErrorCode::InvalidCommand,
                message: caps.image_reason.unwrap_or_default(),
            });
        }
        Ok(())
    }
}

impl Harness {
    pub(crate) async fn open_for_images(
        &self,
        thread: &ThreadId,
        has_images: bool,
    ) -> Result<OpenSession, CoreError> {
        Ok(self.sessions.open_with_images(thread, has_images).await?)
    }
}

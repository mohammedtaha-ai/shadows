impl crate::Code {
    /// `scan`, now, on the test's task: the index a test asks about is the
    /// one its files were just written into.
    pub async fn scan_for_test(
        &self,
        project: &core_projects::ProjectId,
    ) -> Result<(), crate::CodeError> {
        self.scan(project).await
    }

    /// That project's worker drops every `Files` job until shutdown; its
    /// watcher and its periodic scan keep running, so a change the scan
    /// finds proves the scan, not the watcher.
    pub async fn pause_watcher_for_test(&self, project: &core_projects::ProjectId) {
        self.pause_watcher(project).await
    }

    /// The active projects, most recent first.
    pub async fn active_for_test(&self) -> Vec<core_projects::ProjectId> {
        self.active_projects().await
    }

    /// `shut_down`: every worker stops after the file it is on.
    pub async fn shut_down_for_test(&self) {
        self.shut_down().await
    }
}

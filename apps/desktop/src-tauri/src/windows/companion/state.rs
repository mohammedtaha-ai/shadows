#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionState {
    pub revision: u64,
    pub visible: bool,
    pub render_allowed: bool,
    pub paused: bool,
    pub error: Option<String>,
    pub presence: crate::presence::projection::PresenceProjection,
}
pub struct Model {
    pub view: CompanionState,
    pub requested: bool,
    pub initialized: bool,
    pub generation: u64,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            view: CompanionState {
                revision: 0,
                visible: false,
                render_allowed: false,
                paused: false,
                error: None,
                presence: crate::presence::projection::PresenceProjection::unbound(),
            },
            requested: true,
            initialized: false,
            generation: 0,
        }
    }
}
impl Model {
    fn change(&mut self, mutate: impl FnOnce(&mut CompanionState)) {
        let before = self.view.clone();
        mutate(&mut self.view);
        if self.view != before {
            self.view.revision += 1;
        }
    }
    pub fn present(&mut self, presence: crate::presence::projection::PresenceProjection) {
        self.change(|view| view.presence = presence);
    }
    pub fn begin_document(&mut self) -> u64 {
        self.generation += 1;
        self.initialized = false;
        self.change(|v| {
            v.render_allowed = false;
            v.error = None;
        });
        self.generation
    }
    pub fn request_visible(&mut self, visible: bool) {
        self.requested = visible;
    }
    pub fn initialize(&mut self, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        self.initialized = true;
        true
    }
    pub fn deny_render(&mut self) {
        self.change(|v| v.render_allowed = false);
    }
    pub fn finish(&mut self, visible: bool, error: Option<String>) {
        let allowed = visible && self.initialized;
        self.change(|v| {
            v.visible = visible;
            v.render_allowed = allowed;
            v.error = error;
        });
    }
    pub fn pause(&mut self, paused: bool) {
        self.change(|v| v.paused = paused);
    }
    pub fn fail(&mut self, visible: bool) {
        self.initialized = false;
        self.finish(visible, Some("companion_render_failed".into()));
    }
    pub fn expire(&mut self, generation: u64) {
        if generation == self.generation && !self.initialized {
            self.change(|v| {
                v.error = Some("companion_unavailable".into());
                v.render_allowed = false;
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn companion_drawing_failure_revokes_permission_and_is_retryable() {
        let mut model = Model::default();
        let generation = model.begin_document();
        model.initialize(generation);
        model.finish(true, None);
        model.fail(true);
        assert!(model.view.visible);
        assert!(!model.initialized && !model.view.render_allowed);
        assert_eq!(model.view.error.as_deref(), Some("companion_render_failed"));
    }
    #[test]
    fn companion_initially_denies_drawing_and_hide_before_ready_wins() {
        let mut model = Model::default();
        assert!(!model.view.visible && !model.view.render_allowed && !model.view.paused);
        let generation = model.begin_document();
        model.request_visible(false);
        assert!(model.initialize(generation));
        assert!(!model.requested);
        model.finish(false, None);
        assert!(!model.view.render_allowed);
    }
    #[test]
    fn companion_failed_hide_reconciles_actual_visibility() {
        let mut model = Model::default();
        let generation = model.begin_document();
        model.initialize(generation);
        model.finish(true, None);
        let before = model.view.revision;
        model.deny_render();
        assert!(!model.view.render_allowed);
        model.finish(true, Some("companion_hide_failed".into()));
        assert!(model.view.visible && model.view.render_allowed);
        assert!(model.view.revision > before);
        assert!(model.view.error.is_some());
    }
    #[test]
    fn companion_initialization_deadline_is_retryable_and_generation_scoped() {
        let mut model = Model::default();
        let first = model.begin_document();
        model.expire(first);
        assert!(model.view.error.is_some());
        let second = model.begin_document();
        assert!(!model.initialize(first));
        model.expire(first);
        assert!(model.view.error.is_none());
        assert!(model.initialize(second));
        model.finish(true, None);
        assert!(model.view.render_allowed);
        model.expire(second);
        assert!(model.view.error.is_none());
    }
}

#[cfg(test)]
mod presence_tests {
    use super::*;
    #[test]
    fn presence_publication_never_grants_render_permission_or_clears_render_failure() {
        let mut model = Model::default();
        model.fail(false);
        let error = model.view.error.clone();
        model.present(crate::presence::projection::PresenceProjection {
            expression: "success".into(),
            status: "Reply ready".into(),
            preview: None,
        });
        assert!(!model.view.render_allowed);
        assert!(!model.view.visible);
        assert_eq!(model.view.error, error);
    }
}

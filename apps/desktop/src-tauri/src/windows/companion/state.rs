#[cfg(test)]
use super::SurfacePreview;
use super::{SurfaceAgent, SurfaceAppearance, SurfacePanel, SurfacePresence};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingOption {
    pub option_id: String,
    pub name: String,
    pub kind: String,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingRequest {
    pub thread_id: String,
    pub request_id: String,
    pub title: String,
    pub options: Vec<WaitingOption>,
}
/// Requests waiting for the person: the count across AYLUL, the bound conversation's own.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Waiting {
    pub count: u32,
    pub bound: Vec<WaitingRequest>,
}
impl Waiting {
    pub fn is_empty(&self) -> bool {
        self.count == 0 && self.bound.is_empty()
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerView {
    pub open: bool,
    pub revision: u64,
    pub scope: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionState {
    pub revision: u64,
    pub composer: ComposerView,
    pub visible: bool,
    pub render_allowed: bool,
    pub paused: bool,
    pub error: Option<String>,
    pub presence: SurfacePresence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<SurfaceAppearance>,
    #[serde(skip_serializing_if = "SurfacePanel::is_empty")]
    pub panel: SurfacePanel,
    #[serde(skip_serializing_if = "Waiting::is_empty")]
    pub waiting: Waiting,
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
                appearance: None,
                panel: SurfacePanel::default(),
                waiting: Waiting::default(),
                composer: ComposerView::default(),
                visible: false,
                render_allowed: false,
                paused: false,
                error: None,
                presence: SurfacePresence::default(),
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
    pub fn present(
        &mut self,
        mut presence: SurfacePresence,
        appearance: Option<SurfaceAppearance>,
    ) {
        let waiting = std::mem::take(&mut presence.waiting);
        self.change(|view| {
            view.waiting = waiting;
            view.presence = presence;
            view.appearance = appearance;
        });
    }
    pub fn begin_document(&mut self) -> u64 {
        self.blur();
        self.generation += 1;
        self.initialized = false;
        self.change(|v| {
            v.render_allowed = false;
            v.error = None;
        });
        self.generation
    }
    pub fn input(&mut self, open: bool) {
        self.change(|v| {
            if open {
                v.panel.open = false;
            }
            if v.composer.open != open {
                v.composer.open = open;
                v.composer.revision += 1;
            }
        });
    }
    pub fn panel(&mut self, open: bool, agents: Vec<SurfaceAgent>) {
        self.change(|view| {
            view.panel.open = open;
            if open {
                view.panel.agents = agents;
                if view.composer.open {
                    view.composer.open = false;
                    view.composer.revision += 1;
                }
            }
        });
    }
    pub fn blur(&mut self) {
        self.change(|view| view.panel.open = false);
    }
    pub fn composer_context(
        &mut self,
        scope: Option<String>,
        context: Option<String>,
        agent: Option<String>,
    ) {
        self.change(|v| {
            for entry in &mut v.panel.agents {
                entry.bound = agent.as_deref() == Some(entry.slug.as_str());
            }
            if (
                v.composer.scope.as_ref(),
                v.composer.context.as_ref(),
                v.composer.agent.as_ref(),
            ) != (scope.as_ref(), context.as_ref(), agent.as_ref())
            {
                v.composer.scope = scope;
                v.composer.context = context;
                v.composer.agent = agent;
                v.composer.revision += 1;
            }
        });
    }
    pub fn request_visible(&mut self, visible: bool) {
        self.requested = visible;
        if !visible {
            self.input(false);
            self.blur();
        }
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
    fn panel_closes_on_blur_hide_and_editor_open_without_granting_rendering() {
        let mut model = Model::default();
        model.panel(true, vec![]);
        assert!(model.view.panel.open);
        assert!(!model.view.render_allowed);
        model.blur();
        assert!(!model.view.panel.open);
        model.panel(true, vec![]);
        model.input(true);
        assert!(!model.view.panel.open);
        model.panel(true, vec![]);
        model.request_visible(false);
        assert!(!model.view.panel.open);
    }
    #[test]
    fn panel_wire_is_pinned_and_binding_changes_mark_the_selected_agent() {
        let mut model = Model::default();
        model.panel(
            true,
            vec![SurfaceAgent {
                slug: "agent-a".into(),
                name: "AYLUL".into(),
                description: "Builds".into(),
                color: "blue".into(),
                outfit: "headphones".into(),
                bound: false,
            }],
        );
        assert_eq!(
            serde_json::to_value(&model.view).unwrap()["panel"],
            serde_json::json!({
                "open":true,"agents":[{"slug":"agent-a","name":"AYLUL","description":"Builds",
                    "color":"blue","outfit":"headphones","bound":false}]
            })
        );
        model.composer_context(
            Some("scope".into()),
            Some("context".into()),
            Some("agent-a".into()),
        );
        assert!(model.view.panel.agents[0].bound);
        model.composer_context(None, None, None);
        assert!(!model.view.panel.agents[0].bound);
    }
    #[test]
    fn hide_closes_input_and_unhide_keeps_it_closed() {
        let mut model = Model::default();
        model.view.composer.open = true;
        let before = model.view.revision;
        model.request_visible(false);
        assert!(!model.view.composer.open);
        assert!(model.view.revision > before);
        model.request_visible(true);
        assert!(!model.view.composer.open);
    }
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
        model.present(
            SurfacePresence {
                expression: "success".into(),
                status: "Reply ready".into(),
                preview: None,
                waiting: Default::default(),
            },
            None,
        );
        assert!(!model.view.render_allowed);
        assert!(!model.view.visible);
        assert_eq!(model.view.error, error);
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;
    #[test]
    fn appearance_changes_revision_without_render_permission() {
        let mut model = Model::default();
        model.fail(false);
        model.pause(true);
        let before = model.view.clone();
        let presence = before.presence.clone();
        model.present(
            presence.clone(),
            Some(SurfaceAppearance {
                color: "teal".into(),
                outfit: None,
            }),
        );
        assert_eq!(model.view.revision, before.revision + 1);
        assert_eq!(model.view.error, before.error);
        assert_eq!(model.view.composer, before.composer);
        assert_eq!(model.view.paused, before.paused);
        assert!(!model.view.render_allowed);
        assert!(!model.view.visible);
        assert!(serde_json::to_value(&model.view).unwrap()["appearance"]
            .get("outfit")
            .is_none());
        let revision = model.view.revision;
        model.present(
            presence.clone(),
            Some(SurfaceAppearance {
                color: "teal".into(),
                outfit: Some("opaque outfit".into()),
            }),
        );
        assert_eq!(model.view.revision, revision + 1);
        assert!(!model.view.render_allowed);
        model.present(presence, None);
        assert!(serde_json::to_value(&model.view)
            .unwrap()
            .get("appearance")
            .is_none());
    }
}

#[cfg(test)]
mod wire_tests {
    use super::*;

    #[test]
    fn companion_surface_wire_is_pinned() {
        let mut model = Model::default();
        assert_eq!(
            serde_json::to_string(&model.view).unwrap(),
            r#"{"revision":0,"composer":{"open":false,"revision":0,"scope":null,"context":null,"agent":null},"visible":false,"renderAllowed":false,"paused":false,"error":null,"presence":{"expression":"idle","status":"Choose an agent","preview":null}}"#
        );
        model.composer_context(
            Some("scope-1".into()),
            Some("context-1".into()),
            Some("agent-a".into()),
        );
        model.present(
            SurfacePresence {
                expression: "idle".into(),
                status: "Ready".into(),
                preview: None,
                waiting: Default::default(),
            },
            None,
        );
        assert_eq!(
            serde_json::to_string(&model.view).unwrap(),
            r#"{"revision":2,"composer":{"open":false,"revision":1,"scope":"scope-1","context":"context-1","agent":"agent-a"},"visible":false,"renderAllowed":false,"paused":false,"error":null,"presence":{"expression":"idle","status":"Ready","preview":null}}"#
        );
        model.input(true);
        let generation = model.begin_document();
        model.initialize(generation);
        model.finish(true, None);
        model.pause(true);
        model.present(
            SurfacePresence {
                expression: "success".into(),
                status: "Reply ready".into(),
                preview: Some(SurfacePreview {
                    id: "reply-1".into(),
                    text: "مرحبا \"world\"\n".into(),
                    expires_at_ms: u64::MAX,
                }),
                waiting: Default::default(),
            },
            Some(SurfaceAppearance {
                color: "opaque(🌿)".into(),
                outfit: Some("opaque outfit".into()),
            }),
        );
        assert_eq!(
            serde_json::to_string(&model.view).unwrap(),
            r#"{"revision":6,"composer":{"open":true,"revision":2,"scope":"scope-1","context":"context-1","agent":"agent-a"},"visible":true,"renderAllowed":true,"paused":true,"error":null,"presence":{"expression":"success","status":"Reply ready","preview":{"id":"reply-1","text":"مرحبا \"world\"\n","expiresAtMs":18446744073709551615}},"appearance":{"color":"opaque(🌿)","outfit":"opaque outfit"}}"#
        );
        model.input(false);
        model.fail(true);
        model.present(Model::default().view.presence, None);
        assert_eq!(
            serde_json::to_string(&model.view).unwrap(),
            r#"{"revision":9,"composer":{"open":false,"revision":3,"scope":"scope-1","context":"context-1","agent":"agent-a"},"visible":true,"renderAllowed":false,"paused":true,"error":"companion_render_failed","presence":{"expression":"idle","status":"Choose an agent","preview":null}}"#
        );
    }
}

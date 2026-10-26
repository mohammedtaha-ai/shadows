use serde::Serialize;
use std::sync::Mutex;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MainDestination {
    CompanionSettings,
    AgentDraft {
        agent: String,
    },
    Conversation {
        project_id: String,
        thread_id: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingNavigation {
    pub revision: u64,
    pub destination: MainDestination,
}
#[derive(Default)]
pub struct Navigation {
    inner: Mutex<(u64, Option<PendingNavigation>)>,
}
impl Navigation {
    pub fn open_main(
        &self,
        app: &tauri::AppHandle,
        destination: MainDestination,
    ) -> Result<(), String> {
        use tauri::Emitter;
        let pending = self.request(destination);
        let _ = app.emit_to("main", "main-navigation", pending);
        crate::windows::open_main(app).map_err(|_| "main_open_failed".into())
    }

    pub fn request(&self, destination: MainDestination) -> PendingNavigation {
        let mut inner = self.inner.lock().unwrap();
        inner.0 += 1;
        let next = PendingNavigation {
            revision: inner.0,
            destination,
        };
        inner.1 = Some(next.clone());
        next
    }
    pub fn snapshot(&self) -> Option<PendingNavigation> {
        self.inner.lock().unwrap().1.clone()
    }
    pub fn ack(&self, revision: u64) {
        let mut inner = self.inner.lock().unwrap();
        if inner
            .1
            .as_ref()
            .is_some_and(|pending| pending.revision == revision)
        {
            inner.1 = None;
        }
    }
}
#[cfg(test)]
mod navigation_tests {
    use super::*;
    #[test]
    fn old_ack_does_not_clear_a_newer_request() {
        let navigation = Navigation::default();
        let first = navigation.request(MainDestination::CompanionSettings);
        let second = navigation.request(MainDestination::AgentDraft {
            agent: "agent-b".into(),
        });
        navigation.ack(first.revision);
        assert_eq!(navigation.snapshot(), Some(second.clone()));
        navigation.ack(second.revision);
        assert_eq!(navigation.snapshot(), None);
    }
}

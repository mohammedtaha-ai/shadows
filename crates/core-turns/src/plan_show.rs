//! Plan cards composed with the sending turn's live tab.
use super::Turns;
use crate::{Grant, TurnsError};
use core_plans::UiSignal;
use core_plans::{PlanShow, PlanShown};

impl Turns {
    pub async fn show_plan(&self, grant: &Grant, args: PlanShow) -> Result<PlanShown, TurnsError> {
        let (shown, operation, thread) = self.plans.show(grant, args).await?;
        let tab = self.handles.client_tab_for(&operation).await;
        if !shown.replayed {
            let _ = self.ui.send(UiSignal {
                thread_id: thread,
                target_tab: tab,
                workflow_id: shown.workflow_id.clone(),
                version: shown.version,
                task_number: shown.task_number,
                place: shown.place,
            });
        }
        Ok(shown)
    }
}

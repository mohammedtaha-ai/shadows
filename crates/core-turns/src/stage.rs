//! Stage facts supplied to both normal and steered Planner prompts.
use crate::Turns;
use core_db::{Storage, StorageError};
use core_threads::{ThreadId, Threads};

impl Turns {
    pub(super) async fn stage_line(
        storage: &Storage,
        thread: &ThreadId,
    ) -> Result<String, StorageError> {
        let mut conn = storage.reader().begin().await?;
        let context = Threads::turn_context_in(&mut conn, thread).await?;
        // The stage guides a Planner's conversation; an executor works one task.
        if context.harness == shadows_agent::policy::AYLUL
            || context.kind == core_threads::ThreadKind::Executor
        {
            return Ok(String::new());
        }
        core_design::Design::stage_line_in(&mut conn, &context.project_id).await
    }
}

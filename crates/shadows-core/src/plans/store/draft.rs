//! One job: starting a plan version (spec §13.2, §13.6) — the first version
//! of a thread's plan, the next one copied from a frozen version, or the
//! Draft already there. `DraftStarted` names the version, which never changes
//! as the draft is edited, so a replay answers the same. Changing a version
//! is `edit.rs`.

use sqlx::SqliteConnection;

use super::read::{latest_version, load_plan};
use super::task::write_content;
use crate::command::{CommandContext, Writer};
use crate::db::{Storage, StorageError, append_event, classify, now, record_command};
use crate::events::DurableEvent;
use crate::grants::{bind_draft_ref, check_writer};
use crate::plans::model::{DraftStarted, PlanContent, WorkflowId, WorkflowState};
use crate::plans::rules::Problem;
use crate::projects::ProjectId;
use crate::threads::{CreatedTitle, ThreadId, insert_thread};
use shadows_agent::policy;

impl Storage {
    /// §13.6 draft_start in a thread: v1 from `fresh` when the thread has no
    /// plan; a copy of the latest frozen version otherwise (`fresh` ignored);
    /// the existing Draft, unchanged, when there is one. A `source`, when
    /// given, must be that latest version; it is checked after the replay,
    /// because the version a start answered is no longer the latest one.
    pub async fn start_draft(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        thread: &ThreadId,
        source: Option<&WorkflowId>,
        fresh: Option<(&str, &str)>,
        draft_ref: Option<&str>,
    ) -> Result<DraftStarted, StorageError> {
        let (ctx, writer, thread, ts) = (ctx.clone(), writer.clone(), thread.clone(), now());
        let source = source.cloned();
        let fresh = fresh.map(|(t, g)| (t.to_string(), g.to_string()));
        let draft_ref = draft_ref.map(str::to_string);
        self.write_txn(move |conn| {
            Box::pin(async move {
                let project: String =
                    sqlx::query_scalar("SELECT project_id FROM planning_thread WHERE id = ?")
                        .bind(thread.as_str())
                        .fetch_optional(&mut *conn)
                        .await?
                        .ok_or(StorageError::NotFound("planning_thread"))?;
                let project = ProjectId::from_stored(project);
                check_writer(conn, &writer, &project, Some(&thread)).await?;
                if let Some(id) = classify(conn, &ctx, "Thread", thread.as_str()).await? {
                    return started(conn, &WorkflowId::from_stored(id)).await;
                }
                let latest = match latest_version(conn, &thread).await? {
                    Some(id) => Some(load_plan(conn, &id).await?),
                    None => None,
                };
                if let Some(source) = &source
                    && let Some(latest) = &latest
                    && &latest.id != source
                {
                    return Err(StorageError::NotLatestVersion(latest.id.clone()));
                }
                if matches!(writer, Writer::External { .. })
                    && draft_ref.is_none()
                    && !matches!(&latest, Some(plan) if plan.state == WorkflowState::Draft)
                {
                    return Err(StorageError::GrantScope);
                }
                let id = match (latest, fresh) {
                    (Some(draft), _) if draft.state == WorkflowState::Draft => draft.id,
                    (Some(frozen), _) => {
                        let copy = Version {
                            thread: &thread,
                            project: &project,
                            number: frozen.version + 1,
                            previous: Some(&frozen.id),
                        };
                        let before = frozen.content();
                        insert_version(conn, &writer, copy, &before, &ts).await?
                    }
                    (None, Some((title, goal))) => {
                        let first = Version {
                            thread: &thread,
                            project: &project,
                            number: 1,
                            previous: None,
                        };
                        insert_version(conn, &writer, first, &empty(title, goal), &ts).await?
                    }
                    (None, None) => {
                        return Err(StorageError::PlanInvalid(vec![Problem {
                            message: "a plan's first version needs a title and a goal".into(),
                        }]));
                    }
                };
                if let Some(r) = &draft_ref {
                    bind_draft_ref(conn, &writer, r, &id, &project, &ts).await?;
                }
                record_command(
                    conn,
                    &ctx,
                    "Thread",
                    thread.as_str(),
                    "Workflow",
                    id.as_str(),
                    &ts,
                )
                .await?;
                started(conn, &id).await
            })
        })
        .await
    }

    /// §13.6 external draft_start from scratch: a planning thread titled
    /// `title` and its v1, in `project`, harness `claude-code`. Both events
    /// carry the writer as their actor (`planning_thread` has no author).
    pub async fn start_thread_with_draft(
        &self,
        ctx: &CommandContext,
        writer: &Writer,
        project: &ProjectId,
        title: &str,
        goal: &str,
        draft_ref: Option<&str>,
    ) -> Result<DraftStarted, StorageError> {
        let (ctx, writer, project, ts) = (ctx.clone(), writer.clone(), project.clone(), now());
        let (title, goal) = (title.to_string(), goal.to_string());
        let draft_ref = draft_ref.map(str::to_string);
        self.write_txn(move |conn| {
            Box::pin(async move {
                check_writer(conn, &writer, &project, None).await?;
                if matches!(writer, Writer::External { .. }) && draft_ref.is_none() {
                    return Err(StorageError::GrantScope);
                }
                if let Some(id) = classify(conn, &ctx, "Project", project.as_str()).await? {
                    return started(conn, &WorkflowId::from_stored(id)).await;
                }
                let thread = insert_thread(
                    conn,
                    &project,
                    &title,
                    CreatedTitle::Plan,
                    policy::CLAUDE_CODE,
                    writer.actor(),
                    &ts,
                )
                .await?;
                let first = Version {
                    thread: &thread,
                    project: &project,
                    number: 1,
                    previous: None,
                };
                let id = insert_version(conn, &writer, first, &empty(title, goal), &ts).await?;
                if let Some(r) = &draft_ref {
                    bind_draft_ref(conn, &writer, r, &id, &project, &ts).await?;
                }
                record_command(
                    conn,
                    &ctx,
                    "Project",
                    project.as_str(),
                    "Workflow",
                    id.as_str(),
                    &ts,
                )
                .await?;
                started(conn, &id).await
            })
        })
        .await
    }
}

/// Where a new version goes in its thread's chain.
struct Version<'a> {
    thread: &'a ThreadId,
    project: &'a ProjectId,
    number: i64,
    previous: Option<&'a WorkflowId>,
}

fn empty(title: String, goal: String) -> PlanContent {
    PlanContent {
        title,
        goal,
        tasks: Default::default(),
        links: Vec::new(),
    }
}

/// A new Draft at revision 0 holding `content`, journalled as
/// `WorkflowDraftStarted` by the writer.
async fn insert_version(
    conn: &mut SqliteConnection,
    writer: &Writer,
    v: Version<'_>,
    content: &PlanContent,
    ts: &str,
) -> Result<WorkflowId, StorageError> {
    let id = WorkflowId::generate();
    sqlx::query(
        "INSERT INTO workflow
           (id, thread_id, state, previous_version_id, version, revision, title, goal,
            created_at, updated_at)
         VALUES (?,?, 'Draft', ?,?, 0, ?,?,?,?)",
    )
    .bind(id.as_str())
    .bind(v.thread.as_str())
    .bind(v.previous.map(WorkflowId::as_str))
    .bind(v.number)
    .bind(&content.title)
    .bind(&content.goal)
    .bind(ts)
    .bind(ts)
    .execute(&mut *conn)
    .await?;
    write_content(conn, &id, &[], content, ts).await?;
    append_event(
        conn,
        &DurableEvent::new("WorkflowDraftStarted", writer.actor())
            .with_project(v.project)
            .with_thread(v.thread)
            .with_payload(serde_json::json!({ "workflow_id": id, "version": v.number })),
        ts,
    )
    .await?;
    Ok(id)
}

/// What `draft_start` answers for a version.
async fn started(
    conn: &mut SqliteConnection,
    id: &WorkflowId,
) -> Result<DraftStarted, StorageError> {
    let (thread, version): (String, i64) =
        sqlx::query_as("SELECT thread_id, version FROM workflow WHERE id = ?")
            .bind(id.as_str())
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("workflow"))?;
    Ok(DraftStarted {
        workflow_id: id.clone(),
        thread_id: ThreadId::from_stored(thread),
        version,
    })
}

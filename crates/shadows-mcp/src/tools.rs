//! One job: each MCP tool's arguments, and the one service call it makes
//! (spec §13.5–§13.6, §14.5, §15.7): a `Plans` call for the plan tools, a
//! `Code` call for the code tools.
//!
//! What a grant reaches, who writes under it, and which command id a call
//! carries are `Plans`' rules, not a tool's; which projects a code question
//! reads is `Code`'s. `draft_start` and `plan_edit` answer their outcome
//! through `json!`, as they did before `Plans`, so the text a client reads
//! keeps its key order. The code tools answer lines of text (`lines`):
//! formatting is the adapter's job.

use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};
use serde::Deserialize;
use serde_json::json;

use super::refusal::{Refusal, answer, text};
use super::server::Shadows;

use shadows_core::Grant;
use shadows_core::{Answer, Asker, IndexState};
use shadows_core::{DraftStart, Place, PlanEdit, PlanOp, PlanShow, WorkflowId};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanArgs {
    /// Own project by default; a linked project's slug for a read only.
    #[serde(default)]
    project: Option<String>,
    /// The plan version; workflow_list lists each plan's latest version.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskArgs {
    /// As for `workflow_get`.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
    /// The task's number: 4 for T4.
    number: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DraftStartArgs {
    /// The first version's title. Needed only when starting a new plan.
    #[serde(default)]
    title: Option<String>,
    /// The first version's goal. Needed only when starting a new plan.
    #[serde(default)]
    goal: Option<String>,
    /// A plan to start its next version, or none for a new plan.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    plan_id: Option<shadows_core::PlanId>,
    /// Why the plan changes, required for every version after the first.
    #[serde(default)]
    reason: Option<String>,
    /// External agents: the ref `draft_prepare` answered. The same ref always
    /// answers the same plan; a new plan takes a new ref.
    #[serde(default)]
    draft_ref: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WorkflowListArgs {
    /// Own project by default; a linked project's slug for a read only.
    #[serde(default)]
    project: Option<String>,
    /// Include archived plans alongside Active plans.
    #[serde(default)]
    archived: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanEditArgs {
    /// As for `workflow_get`.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
    /// The revision the edit was built on, as `workflow_get` answered it.
    expected_revision: i64,
    /// Applied together or not at all; the revision moves once.
    ops: Vec<PlanOp>,
    /// Leave it out: Shadows derives one, so a retry of the same edit
    /// answers the first result.
    #[serde(default)]
    command_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanShowArgs {
    /// As for `workflow_get`.
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    workflow_id: Option<WorkflowId>,
    /// A task to show, by its number: 4 for T4. Leave it out to show the plan.
    #[serde(default)]
    task_number: Option<u32>,
    /// `inline` as a card in the conversation, `side` in a panel beside it,
    /// `page` on the plan's own page. Every place also leaves a card.
    place: Place,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NameArgs {
    /// The exact name: a function, method, type, trait, class, interface or constant.
    name: String,
    /// A linked project's slug, to ask only it.
    /// Leave it out to ask this project and every linked one.
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct OutlineArgs {
    /// A file or folder, relative to the project's folder. "" for the whole project.
    path: String,
    /// As for where_is.
    #[serde(default)]
    project: Option<String>,
}

#[tool_router(vis = "pub(super)")]
impl Shadows {
    #[tool(
        description = "Where a name is defined: project, file, line, kind and signature. \
                       Never the code; open the file if you need it."
    )]
    async fn where_is(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<NameArgs>,
    ) -> CallToolResult {
        let only = args.project.as_deref();
        let asked = self
            .core
            .code()
            .definitions(Asker::Grant(&grant), only, &args.name);
        text(asked.await.map(|a| lines(&a, false)).map_err(Refusal::from))
    }

    #[tool(description = "Where a name is used, matched by name only: \
                       two things with the same name are not told apart.")]
    async fn who_uses(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<NameArgs>,
    ) -> CallToolResult {
        let only = args.project.as_deref();
        let asked = self
            .core
            .code()
            .references(Asker::Grant(&grant), only, &args.name);
        text(asked.await.map(|a| lines(&a, true)).map_err(Refusal::from))
    }

    #[tool(
        description = "The definitions in a file or folder: file, line, kind, name and signature."
    )]
    async fn outline(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<OutlineArgs>,
    ) -> CallToolResult {
        let only = args.project.as_deref();
        let asked = self
            .core
            .code()
            .outline(Asker::Grant(&grant), only, &args.path);
        text(asked.await.map(|a| lines(&a, false)).map_err(Refusal::from))
    }

    #[tool(
        description = "List plans: each one's latest version. Use project for a linked \
                       project's slug, archived to include archived plans."
    )]
    async fn workflow_list(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<WorkflowListArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .list_for(&grant, args.archived, args.project.as_deref())
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Read a plan version: title, goal, tasks, links, revision, state, \
                       and what blocks approval. Use project for a linked project's slug."
    )]
    async fn workflow_get(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .get_for(&grant, args.workflow_id.as_ref(), args.project.as_deref())
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(description = "Read one task of a plan version by its number.")]
    async fn task_get(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<TaskArgs>,
    ) -> CallToolResult {
        answer(
            self.core
                .plans()
                .task_for(&grant, args.workflow_id.as_ref(), args.number)
                .await
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Get a draft_ref for draft_start. Call it once for each new plan you \
                       intend; a ref not used within an hour expires."
    )]
    async fn draft_prepare(&self, Extension(grant): Extension<Grant>) -> CallToolResult {
        let issued = self.core.plans().prepare_draft(&grant).await;
        answer(
            issued
                .map(|r| json!({ "draft_ref": r }))
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Start a plan version to edit. Give plan_id to continue that plan, \
                       or leave it out to create a new plan from a title and goal. \
                       A later version needs a reason; an existing Draft is returned unchanged."
    )]
    async fn draft_start(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<DraftStartArgs>,
    ) -> CallToolResult {
        let args = DraftStart {
            title: args.title,
            goal: args.goal,
            plan_id: args.plan_id,
            reason: args.reason,
            draft_ref: args.draft_ref,
        };
        answer(
            self.core
                .plans()
                .start_draft(&grant, args)
                .await
                .map(|started| json!(started))
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Edit a draft plan version at expected_revision with a list of \
                       operations: plan_put, task_add, task_update, task_remove, link_put, \
                       link_remove. Put changes made together in one call."
    )]
    async fn plan_edit(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanEditArgs>,
    ) -> CallToolResult {
        let args = PlanEdit {
            workflow_id: args.workflow_id,
            expected_revision: args.expected_revision,
            ops: args.ops,
            command_id: args.command_id,
        };
        answer(
            self.core
                .plans()
                .edit(&grant, args)
                .await
                .map(|outcome| json!(outcome))
                .map_err(Refusal::from),
        )
    }

    #[tool(
        description = "Show the person the plan, or one task of it, while they talk with you: \
                       inline in the conversation, side in a panel beside it, or page on its own \
                       page. Changes no plan."
    )]
    async fn plan_show(
        &self,
        Extension(grant): Extension<Grant>,
        Parameters(args): Parameters<PlanShowArgs>,
    ) -> CallToolResult {
        let args = PlanShow {
            workflow_id: args.workflow_id,
            task_number: args.task_number,
            place: args.place,
        };
        answer(
            self.core
                .plans()
                .show(&grant, args)
                .await
                .map_err(Refusal::from),
        )
    }
}

/// A code answer as text (§15.7): one line per hit, `{project}: {path}:{line}
/// {kind} {name} — {signature}`, or `(matched by name)` in place of the
/// signature for `references`; then `more`, the suggestions, and one line
/// per project's status.
fn lines(answer: &Answer, references: bool) -> String {
    let mut out = Vec::new();
    for h in &answer.hits {
        let at = format!("{}: {}:{} {} {}", h.project, h.path, h.line, h.kind, h.name);
        out.push(match (&h.signature, references) {
            (_, true) => format!("{at} (matched by name)"),
            (Some(signature), false) => format!("{at} — {signature}"),
            (None, false) => at,
        });
    }
    if answer.hits.is_empty() {
        out.push("nothing found".into());
    }
    if answer.more {
        out.push("more results: narrow the name or the path".into());
    }
    if !answer.suggestions.is_empty() {
        out.push(format!("did you mean: {}", answer.suggestions.join(", ")));
    }
    for s in &answer.status {
        let state = match &s.state {
            IndexState::Ready => "ready".to_string(),
            IndexState::Indexing { done, found } => format!("indexing {done}/{found}"),
            IndexState::Inactive => "inactive".into(),
            IndexState::NoDirectory => "not indexed: no directory".into(),
            IndexState::DirectoryMissing => "directory missing".into(),
        };
        let skipped: Vec<String> = s
            .skipped
            .iter()
            .map(|k| format!("{} {}", k.count, k.reason))
            .collect();
        if skipped.is_empty() {
            out.push(format!("{}: {state}", s.project));
        } else {
            out.push(format!(
                "{}: {state}; skipped {}",
                s.project,
                skipped.join(", ")
            ));
        }
    }
    out.join("\n")
}

//! The bounded design context of a task in an approved version.
use crate::{PlanId, Plans, PlansError, TaskContent, WorkflowId};
use core_db::Tx;
use core_design::{LayoutPart, OwnedVia, PartId, ProjectLayout};
use core_grants::Grant;
use core_projects::ProjectId;
use serde::{Deserialize, Serialize};

// `part` and `via` are typed as open objects in the schema: their own types
// carry no schema derive.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<crate::Phase>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phase_dependencies: Vec<PhaseDependency>,
    pub plan: ContextPlan,
    pub task: TaskContent,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contracts: Vec<crate::ResolvedContract>,
    /// The task contract's pinned decision revisions as Design holds them now.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(required = false)]
    pub decisions: Vec<core_design::DecisionRecord>,
    /// The task contract's pinned table and screen revisions in contract order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(required = false)]
    pub elements: Vec<core_design::DesignElement>,
    pub parts: Vec<TaskPart>,
    pub unowned_paths: Vec<String>,
    pub standards: ContextStandards,
    pub map_revision: i64,
    /// The outcomes the plan serves now; absent from briefs saved before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(required = false)]
    pub outcomes: Vec<crate::DeliveryOutcome>,
    /// The task scope per access mode; absent from briefs saved before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(required = false)]
    pub ownership: Option<TaskOwnership>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PhaseDependency {
    pub reference: crate::PhaseRef,
    pub target: Option<crate::PhaseEndpoint>,
    pub phase: Option<crate::Phase>,
    pub broken: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContextPlan {
    pub plan_id: PlanId,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContextStandards {
    pub base: i64,
    pub additions: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskPart {
    pub part: LayoutPart,
    pub paths: Vec<TaskPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskPath {
    pub path: String,
    pub via: OwnedVia,
}

impl Plans {
    /// The context a grant may read: its project's approved task, after the
    /// grant's own checks.
    pub async fn task_context(
        &self,
        grant: &Grant,
        workflow: Option<&WorkflowId>,
        task: u32,
        phase: Option<&crate::PhaseId>,
    ) -> Result<TaskContext, PlansError> {
        self.store.task_context(grant, workflow, task, phase).await
    }

    /// The one assembly of a task's context, as `tx` sees the approved
    /// version `workflow` of `project` and its layout: what a grant reads and
    /// what an attempt saves inside its own write.
    pub async fn task_context_in(
        tx: &mut Tx,
        project: &ProjectId,
        workflow: &WorkflowId,
        task: u32,
        phase: Option<&crate::PhaseId>,
    ) -> Result<TaskContext, PlansError> {
        crate::store::PlansStore::task_context_in(tx, project, workflow, task, phase).await
    }
}

/// Which parts own each path a task reads and writes, kept per access mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskOwnership {
    pub reads: Vec<ScopePath>,
    pub writes: Vec<ScopePath>,
}

/// One declared scope path and the parts that own it, empty when none does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScopePath {
    pub path: String,
    pub owners: Vec<ScopeOwner>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScopeOwner {
    pub part_id: PartId,
    pub via: OwnedVia,
}

impl TaskOwnership {
    /// The one mapping of a task's scope to the layout's owners: approval,
    /// plan reads, the task context and State all read it from here.
    pub(crate) fn resolve(layout: &ProjectLayout, task: &TaskContent) -> TaskOwnership {
        TaskOwnership {
            reads: Self::list(layout, &task.reads),
            writes: Self::list(layout, &task.writes),
        }
    }

    fn list(layout: &ProjectLayout, paths: &[String]) -> Vec<ScopePath> {
        let mut out: Vec<ScopePath> = Vec::new();
        for path in paths {
            if out.iter().any(|p| p.path == *path) {
                continue;
            }
            let owners = layout
                .owners(path)
                .into_iter()
                .map(|o| ScopeOwner {
                    part_id: o.part,
                    via: o.via,
                })
                .collect();
            out.push(ScopePath {
                path: path.clone(),
                owners,
            });
        }
        out
    }

    /// Reads then writes, each distinct path once.
    pub(crate) fn union(&self) -> Vec<&ScopePath> {
        let mut out: Vec<&ScopePath> = Vec::new();
        for scope in self.reads.iter().chain(&self.writes) {
            if !out.iter().any(|p| p.path == scope.path) {
                out.push(scope);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_design::{LayoutLayer, LayoutPart};

    fn part(id: &PartId, folder: &str, layers: Vec<LayoutLayer>) -> LayoutPart {
        LayoutPart {
            id: id.clone(),
            parent: None,
            title: folder.into(),
            kind: None,
            folder: folder.into(),
            grouping: false,
            layers,
            rules: Vec::new(),
        }
    }

    #[test]
    fn task_ownership_preserves_access_and_nested_owners() {
        let (water, store, infra) = (PartId::generate(), PartId::generate(), PartId::generate());
        let layer = LayoutLayer {
            id: "infrastructure".into(),
            role: "storage".into(),
            place: Some("water/internal/store".into()),
            not_used: None,
            may_use: Vec::new(),
        };
        let layout = ProjectLayout {
            map_revision: 1,
            standards_base: 1,
            standards_additions: None,
            parts: vec![
                part(&water, "water", Vec::new()),
                part(&store, "water/internal/store", Vec::new()),
                part(&infra, "infra", vec![layer]),
            ],
        };
        let shared = "water/internal/store/**".to_string();
        let task = TaskContent {
            number: 1,
            title: "t".into(),
            goal: "g".into(),
            reads: vec![shared.clone(), "outside/**".into(), shared.clone()],
            writes: vec![shared.clone()],
            acceptance: Vec::new(),
            phases: Vec::new(),
            contract: None,
        };
        let ownership = TaskOwnership::resolve(&layout, &task);
        let expected = vec![
            ScopeOwner {
                part_id: store.clone(),
                via: OwnedVia::Folder,
            },
            ScopeOwner {
                part_id: infra.clone(),
                via: OwnedVia::Layer("infrastructure".into()),
            },
        ];
        assert_eq!(
            ownership.reads.len(),
            2,
            "a repeated read path appears once"
        );
        assert_eq!(ownership.reads[0].path, shared);
        assert_eq!(ownership.reads[0].owners, expected);
        assert_eq!(ownership.writes.len(), 1);
        assert_eq!(ownership.writes[0].owners, expected);
        assert_eq!(ownership.reads[1].path, "outside/**");
        assert!(ownership.reads[1].owners.is_empty());
        assert_eq!(ownership.union().len(), 2);
    }
}

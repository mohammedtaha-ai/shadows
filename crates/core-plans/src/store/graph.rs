//! Snapshot checks for dependencies reached through other plans.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use sqlx::SqliteConnection;

use crate::dependency_graph::DependencyGraph;
use crate::{Plan, PlanContent, PlanId, PlanOp, Problem, TaskParent, WorkflowId};
use core_db::StorageError;
use core_projects::ProjectId;

type Owner = (ProjectId, Option<WorkflowId>, Option<String>);

impl GraphRows {
    pub(super) async fn check_parents(
        conn: &mut SqliteConnection,
        source: &Plan,
        ops: &[PlanOp],
    ) -> Result<(), crate::PlansError> {
        for op in ops {
            let PlanOp::LinkPut { link } = op else {
                continue;
            };
            let TaskParent::Plan { plan_id, task } = &link.after else {
                continue;
            };
            let invalid = |message| crate::PlansError::PlanLinkInvalid(message);
            if *plan_id == source.plan_id {
                return Err(invalid(
                    "use a local task number for a link inside this plan".to_string(),
                ));
            }
            let Some((project, version, _)) = GraphRows::owner(conn, plan_id).await? else {
                return Err(invalid(format!("plan {plan_id} T{task} is unavailable")));
            };
            if !codebase::Codebase::project_reachable_in(conn, &source.project_id, &project).await?
            {
                return Err(invalid(format!(
                    "a project link to plan {plan_id}'s project is missing"
                )));
            }
            let Some(version) = version else {
                return Err(invalid(format!("plan {plan_id} has no latest version")));
            };
            let known: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM task WHERE workflow_id=? AND number=?)",
            )
            .bind(version.as_str())
            .bind(task)
            .fetch_one(&mut *conn)
            .await?;
            if !known {
                return Err(invalid(format!(
                    "plan {plan_id} T{task} is missing from its latest version"
                )));
            }
        }
        Ok(())
    }
}

impl GraphRows {
    pub(super) async fn owner(
        conn: &mut SqliteConnection,
        plan: &PlanId,
    ) -> Result<Option<Owner>, StorageError> {
        let row: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT p.project_id, w.id, w.title FROM plan p
           LEFT JOIN workflow w ON w.plan_id=p.id AND w.version=(
             SELECT MAX(version) FROM workflow WHERE plan_id=p.id)
          WHERE p.id=?",
        )
        .bind(plan.as_str())
        .fetch_optional(&mut *conn)
        .await?;
        if let Some((project, _, _)) = &row
            && !core_projects::Projects::is_live_in(
                conn,
                &ProjectId::from_recorded(project.clone()),
            )
            .await?
        {
            return Ok(None);
        }
        Ok(row.map(|(project, version, title)| {
            (
                ProjectId::from_recorded(project),
                version.map(WorkflowId::from_stored),
                title,
            )
        }))
    }
}

pub(super) struct GraphCheck {
    pub(super) problems: Vec<Problem>,
    pub(super) has_phases: bool,
}

impl GraphRows {
    pub(super) async fn blockers(
        conn: &mut SqliteConnection,
        root: &Plan,
        origin: Option<&ProjectId>,
    ) -> Result<Vec<Problem>, StorageError> {
        Ok(Self::check(conn, root, origin, None).await?.problems)
    }

    pub(super) async fn check(
        conn: &mut SqliteConnection,
        root: &Plan,
        origin: Option<&ProjectId>,
        candidate: Option<&PlanContent>,
    ) -> Result<GraphCheck, StorageError> {
        use super::phase_refs::{PhaseRows, VersionSnapshot};
        let mut root_version = VersionSnapshot::from_plan(root);
        if let Some(content) = candidate {
            root_version.content = std::sync::Arc::new(content.clone());
        }
        let external = root_version
            .content
            .links
            .iter()
            .any(|l| matches!(l.after, TaskParent::Plan { .. }))
            || root_version
                .content
                .tasks
                .values()
                .flat_map(|t| &t.phases)
                .flat_map(|p| &p.waits_for)
                .any(|r| matches!(r, crate::PhaseRef::Plan { .. }));
        if !external {
            return Ok(GraphCheck {
                problems: Vec::new(),
                has_phases: root_version
                    .content
                    .tasks
                    .values()
                    .any(|t| !t.phases.is_empty()),
            });
        }
        let root_key = root.id.to_string();
        let mut versions = BTreeMap::from([(root_key.clone(), root_version)]);
        let mut pending = versions[&root_key]
            .content
            .tasks
            .keys()
            .map(|n| (root_key.clone(), *n))
            .collect::<VecDeque<_>>();
        let mut reached = BTreeSet::new();
        let mut problems = Vec::new();
        let mut task_edges = Vec::new();
        let mut phase_edges = Vec::new();
        let mut has_phases = false;
        while let Some((key, number)) = pending.pop_front() {
            if !reached.insert((key.clone(), number)) {
                continue;
            }
            let source = versions[&key].clone();
            let Some(task) = source.content.tasks.get(&number) else {
                continue;
            };
            has_phases |= !task.phases.is_empty();
            for link in source.content.links.iter().filter(|l| l.task == number) {
                let target = match &link.after {
                    TaskParent::Local(_) => source.clone(),
                    TaskParent::Plan { plan_id, task } => {
                        let Some((project, workflow, title)) = Self::owner(conn, plan_id).await?
                        else {
                            problems.push(Problem::new(format!(
                                "T{number} links to unavailable plan {plan_id} T{task}"
                            )));
                            continue;
                        };
                        if !codebase::Codebase::project_reachable_in(
                            conn,
                            &source.project,
                            &project,
                        )
                        .await?
                        {
                            problems.push(Problem::new(format!(
                                "T{number} needs a project link to plan {plan_id} T{task}"
                            )));
                            continue;
                        }
                        let Some(workflow) = workflow else {
                            problems.push(Problem::new(format!("plan {plan_id} has no version")));
                            continue;
                        };
                        let target_key = workflow.to_string();
                        if let Some(version) = versions.get(&target_key) {
                            version.clone()
                        } else if let Some(version) = PhaseRows::load(conn, &workflow).await? {
                            version
                        } else {
                            problems.push(Problem::new(format!(
                                "{} T{task} is unavailable",
                                title.unwrap_or_else(|| plan_id.to_string())
                            )));
                            continue;
                        }
                    }
                };
                let target_key = target.workflow.to_string();
                if !target.content.tasks.contains_key(&link.after.number()) {
                    problems.push(Problem::new(format!(
                        "{} T{} is missing from its latest version",
                        target.title,
                        link.after.number()
                    )));
                    continue;
                }
                pending.push_back((target_key.clone(), link.after.number()));
                task_edges.push((key.clone(), target_key.clone(), link.clone()));
                versions.entry(target_key).or_insert(target);
            }
            for phase in &task.phases {
                for reference in &phase.waits_for {
                    match PhaseRows::resolve(conn, &source, reference).await {
                        Ok((target, number, target_phase)) => {
                            let target_key = target.workflow.to_string();
                            pending.push_back((target_key.clone(), number));
                            phase_edges.push((
                                key.clone(),
                                task.number,
                                phase.id.to_string(),
                                target_key.clone(),
                                number,
                                target_phase.id.to_string(),
                            ));
                            versions.entry(target_key).or_insert(target);
                        }
                        Err(StorageError::Constraint(reason)) => problems.push(Problem::new(
                            format!("T{} · P{}: {reason}", task.number, phase.number),
                        )),
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        let mut graph = DependencyGraph::new();
        for (key, number) in &reached {
            if let Some(task) = versions[key].content.tasks.get(number) {
                graph.phases(key, task);
            }
        }
        for (source, target, link) in task_edges {
            graph.link_to(&source, &target, &link);
        }
        for (source, task, phase, target, number, id) in phase_edges {
            graph.wait((&source, task, &phase), (&target, number, &id));
        }
        if let Some(cycle) = graph.cycle()
            && cycle.iter().any(|event| event.plan != root_key)
        {
            let mut names = Vec::new();
            for event in cycle {
                let version = &versions[&event.plan];
                let title = if codebase::Codebase::project_reachable_in(
                    conn,
                    origin.unwrap_or(&root.project_id),
                    &version.project,
                )
                .await?
                {
                    version.title.clone()
                } else {
                    version.plan_id.to_string()
                };
                names.push(format!(
                    "{title} v{} {}",
                    version.version,
                    event.task_label()
                ));
            }
            problems.push(Problem::new(format!(
                "these links form a cycle: {}",
                names.join(", ")
            )));
        }
        Ok(GraphCheck {
            problems,
            has_phases,
        })
    }
}

pub(crate) struct GraphRows;

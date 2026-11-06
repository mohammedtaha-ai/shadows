//! Ordered phase contracts owned by Plans.
use crate::{PlanContent, PlanId, Problem};
use core_db::newtype_id;

newtype_id! { PhaseId }

impl PhaseId {
    pub fn from_recorded(id: String) -> Self {
        Self::from_stored(id)
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct Phase {
    #[schemars(with = "String")]
    pub id: PhaseId,
    pub number: u32,
    pub title: String,
    pub goal: String,
    pub acceptance: Vec<u32>,
    pub waits_for: Vec<PhaseRef>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PhaseRef {
    Local {
        task: u32,
        #[schemars(with = "String")]
        phase: PhaseId,
    },
    Plan {
        #[schemars(with = "String")]
        plan_id: PlanId,
        version: i64,
        task: u32,
        #[schemars(with = "String")]
        phase: PhaseId,
    },
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
pub struct PhaseInput {
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub id: Option<PhaseId>,
    pub title: String,
    pub goal: String,
    pub acceptance: Vec<u32>,
    pub waits_for: Vec<PhaseRef>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PhaseEndpoint {
    pub plan_id: PlanId,
    pub workflow_id: crate::WorkflowId,
    pub version: i64,
    pub plan_title: String,
    pub task: u32,
    pub phase_id: PhaseId,
    pub phase_number: u32,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct LinkedPhase {
    pub source: PhaseEndpoint,
    pub reference: PhaseRef,
    pub target: Option<PhaseEndpoint>,
    pub broken: Option<String>,
}

pub(super) struct PhaseRules;

impl PhaseRules {
    pub(super) fn problems(content: &PlanContent) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (&number, task) in &content.tasks {
            let mut ids = std::collections::HashSet::new();
            for (index, phase) in task.phases.iter().enumerate() {
                let name = format!("T{number} · P{}", index + 1);
                if phase.number as usize != index + 1 {
                    problems.push(Problem::new(format!(
                        "{name}: phase numbers follow list order"
                    )));
                }
                if phase.id.as_str().trim().is_empty() || !ids.insert(phase.id.as_str()) {
                    problems.push(Problem::new(format!(
                        "{name}: missing or repeated phase id"
                    )));
                }
                if phase.title.trim().is_empty() || phase.goal.trim().is_empty() {
                    problems.push(Problem::new(format!(
                        "{name}: a phase needs a title and goal"
                    )));
                }
                for item in &phase.acceptance {
                    if !task.acceptance.iter().any(|a| a.number == *item) {
                        problems.push(Problem::new(format!("{name}: no acceptance item {item}")));
                    }
                }
                for target in &phase.waits_for {
                    match target {
                        PhaseRef::Local { task: n, phase: id } => {
                            if *n == number && *id == phase.id {
                                problems
                                    .push(Problem::new(format!("{name} cannot wait for itself")));
                            } else if !content
                                .tasks
                                .get(n)
                                .is_some_and(|t| t.phases.iter().any(|p| p.id == *id))
                            {
                                problems.push(Problem::new(format!(
                                    "{name} waits for missing T{n} phase {id}"
                                )));
                            }
                        }
                        PhaseRef::Plan {
                            plan_id,
                            version,
                            task,
                            phase,
                        } => {
                            if *version < 1
                                || *task == 0
                                || plan_id.as_str().is_empty()
                                || phase.as_str().is_empty()
                            {
                                problems.push(Problem::new(format!(
                                    "{name}: a phase pin needs a plan, positive version/task and phase"
                                )));
                            }
                        }
                    }
                }
            }
        }
        problems
    }
}

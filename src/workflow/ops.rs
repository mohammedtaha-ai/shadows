//! One job: applying one batch of plan edits (spec §13.4, "A batch").
//!
//! The batch is refused as a whole or applied as a whole. What can be judged
//! from the operations alone is refused first; otherwise every operation is
//! applied in order and the final state is checked, so a task can be added and
//! linked in one call.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::check::{Problem, edit_problems_after_removing};
use super::{Link, LinkKind, PlanContent, TaskContent, link_name};

/// One edit operation. The names are the `plan_edit` tool's, exactly.
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PlanOp {
    PlanPut {
        title: String,
        goal: String,
    },
    TaskAdd {
        task: TaskContent,
    },
    TaskUpdate {
        task: TaskContent,
    },
    TaskRemove {
        number: u32,
    },
    /// Adds the link, or replaces the one with the same `task`, `after` and
    /// `kind` (§13.3: at most one link of each kind between two tasks).
    LinkPut {
        link: Link,
    },
    LinkRemove {
        task: u32,
        after: u32,
        kind: LinkKind,
    },
}

/// A batch that held: the new content, and what it touched.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Applied {
    pub content: PlanContent,
    /// Sorted, without repeats: every task a `task_*` op named and both ends
    /// of every `link_*` op.
    pub changed_tasks: Vec<u32>,
    /// One line a person can read, e.g. `2 changes: added T4, linked T4 → T2`.
    pub summary: String,
}

/// What one operation acts on. Two operations on one object in a batch are
/// refused rather than resolved by their order (§13.4).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Object {
    Plan,
    Task(u32),
    Link(u32, u32, LinkKind),
}

impl Object {
    /// A link's identity: at most one link of each kind between two tasks (§13.3).
    fn of_link(l: &Link) -> Self {
        Object::Link(l.task, l.after, l.kind)
    }

    fn name(self) -> String {
        match self {
            Object::Plan => "the plan".to_string(),
            Object::Task(n) => format!("T{n}"),
            Object::Link(task, after, kind) => link_name(task, after, kind),
        }
    }
}

impl PlanOp {
    fn object(&self) -> Object {
        match self {
            PlanOp::PlanPut { .. } => Object::Plan,
            PlanOp::TaskAdd { task } | PlanOp::TaskUpdate { task } => Object::Task(task.number),
            PlanOp::TaskRemove { number } => Object::Task(*number),
            PlanOp::LinkPut { link } => Object::of_link(link),
            PlanOp::LinkRemove { task, after, kind } => Object::Link(*task, *after, *kind),
        }
    }

    /// Every task number the operation names.
    fn numbers(&self) -> Vec<u32> {
        match self {
            PlanOp::PlanPut { .. } => vec![],
            PlanOp::TaskAdd { task } | PlanOp::TaskUpdate { task } => vec![task.number],
            PlanOp::TaskRemove { number } => vec![*number],
            PlanOp::LinkPut { link } => vec![link.task, link.after],
            PlanOp::LinkRemove { task, after, .. } => vec![*task, *after],
        }
    }
}

/// Applies `ops` in order to a copy of `current`. Refused, with every problem
/// found, when an operation cannot apply or the final state breaks a rule of
/// §13.4; `current` is never changed.
pub fn apply(current: &PlanContent, ops: &[PlanOp]) -> Result<Applied, Vec<Problem>> {
    let refusals = refusals(current, ops);
    if !refusals.is_empty() {
        return Err(refusals);
    }

    let mut content = current.clone();
    let mut parts = Vec::with_capacity(ops.len());
    let mut changed = BTreeSet::new();
    let mut removed = BTreeSet::new();
    for op in ops {
        changed.extend(op.numbers());
        parts.push(apply_one(&mut content, op, &mut removed));
    }

    let problems = edit_problems_after_removing(&content, &removed);
    if !problems.is_empty() {
        return Err(problems);
    }
    let count = match ops.len() {
        1 => "1 change".to_string(),
        n => format!("{n} changes"),
    };
    Ok(Applied {
        content,
        changed_tasks: changed.into_iter().collect(),
        summary: format!("{count}: {}", parts.join(", ")),
    })
}

/// Applies one operation already known to apply, and says what it did.
fn apply_one(content: &mut PlanContent, op: &PlanOp, removed: &mut BTreeSet<u32>) -> String {
    match op {
        PlanOp::PlanPut { title, goal } => {
            content.title = title.clone();
            content.goal = goal.clone();
            "renamed the plan".to_string()
        }
        PlanOp::TaskAdd { task } => {
            content.tasks.insert(task.number, task.clone());
            format!("added T{}", task.number)
        }
        PlanOp::TaskUpdate { task } => {
            content.tasks.insert(task.number, task.clone());
            format!("updated T{}", task.number)
        }
        PlanOp::TaskRemove { number } => {
            content.tasks.remove(number);
            removed.insert(*number);
            format!("removed T{number}")
        }
        PlanOp::LinkPut { link } => {
            match content
                .links
                .iter_mut()
                .find(|l| Object::of_link(l) == op.object())
            {
                Some(existing) => *existing = link.clone(),
                None => content.links.push(link.clone()),
            }
            format!("linked T{} → T{}", link.task, link.after)
        }
        PlanOp::LinkRemove { task, after, .. } => {
            content.links.retain(|l| Object::of_link(l) != op.object());
            format!("unlinked T{task} → T{after}")
        }
    }
}

/// What can be refused from the operations and `current` alone, in op order.
/// An object named twice is reported once, and its operations are not judged
/// further: which of them the writer meant is exactly what is unknown.
fn refusals(current: &PlanContent, ops: &[PlanOp]) -> Vec<Problem> {
    if ops.is_empty() {
        return vec![Problem::new("an edit needs at least one operation")];
    }
    let mut uses: HashMap<Object, usize> = HashMap::new();
    for op in ops {
        *uses.entry(op.object()).or_default() += 1;
    }
    let mut reported = HashSet::new();
    let mut problems = Vec::new();
    for op in ops {
        let object = op.object();
        if uses[&object] > 1 {
            if reported.insert(object) {
                problems.push(Problem::new(format!(
                    "two operations on {} in one edit",
                    object.name()
                )));
            }
        } else if let Some(message) = refusal(current, op) {
            problems.push(Problem::new(message));
        }
    }
    problems
}

fn refusal(current: &PlanContent, op: &PlanOp) -> Option<String> {
    if op.numbers().contains(&0) {
        return Some("T0 is not a task number; numbers start at 1".to_string());
    }
    let exists = |n: &u32| current.tasks.contains_key(n);
    match op {
        PlanOp::TaskAdd { task } if exists(&task.number) => Some(format!(
            "T{} already exists; use task_update to change it",
            task.number
        )),
        PlanOp::TaskUpdate {
            task: TaskContent { number, .. },
        }
        | PlanOp::TaskRemove { number }
            if !exists(number) =>
        {
            Some(format!("T{number} does not exist in this plan"))
        }
        PlanOp::LinkRemove { task, after, kind }
            if !current
                .links
                .iter()
                .any(|l| Object::of_link(l) == op.object()) =>
        {
            Some(format!(
                "{} does not exist",
                link_name(*task, *after, *kind)
            ))
        }
        _ => None,
    }
}

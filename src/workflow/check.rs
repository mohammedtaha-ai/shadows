//! One job: what makes a plan invalid or unready (spec §13.4).
//!
//! One validator answers both levels, so the web client's list of what blocks
//! approval and the approval route's refusal never disagree. Problems come in a
//! stable order — tasks ascending, then links in stored order, then a cycle —
//! so the same plan always yields the same list.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::{algo::tarjan_scc, graph::DiGraph};

use super::{Link, LinkKind, PlanContent, link_name};

/// One broken or missing thing, named so a person can act on it: tasks as `T4`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Problem {
    pub message: String,
}

impl Problem {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// What every edit's final state must hold (§13.4, "Every edit"). Empty means
/// the content may be stored as a `Draft`.
pub fn edit_problems(content: &PlanContent) -> Vec<Problem> {
    edit_problems_after_removing(content, &BTreeSet::new())
}

/// [`edit_problems`], plus what a plan needs before it can be approved (§13.4,
/// "Approval"). Empty means the content may be frozen.
pub fn approval_problems(content: &PlanContent) -> Vec<Problem> {
    let mut problems = edit_problems(content);
    if content.title.trim().is_empty() {
        problems.push(Problem::new("the plan has no title"));
    }
    if content.tasks.is_empty() {
        problems.push(Problem::new("the plan has no tasks"));
    }
    for (n, task) in &content.tasks {
        if task.goal.trim().is_empty() {
            problems.push(Problem::new(format!("T{n} has no goal")));
        }
        if task.acceptance.is_empty() {
            problems.push(Problem::new(format!("T{n} has no acceptance item")));
        }
    }
    problems
}

/// [`edit_problems`] for the state an edit left behind: a link still naming a
/// task the same edit removed says so ("T4 is still linked from T2"), instead
/// of calling it a task that never existed. Nothing is removed implicitly.
pub(super) fn edit_problems_after_removing(
    content: &PlanContent,
    removed: &BTreeSet<u32>,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (&n, task) in &content.tasks {
        if task.number != n {
            problems.push(Problem::new(format!(
                "the task stored as T{n} is numbered T{}",
                task.number
            )));
        }
        let mut seen = BTreeSet::new();
        let repeated: BTreeSet<u32> = task
            .acceptance
            .iter()
            .filter(|item| !seen.insert(item.number))
            .map(|item| item.number)
            .collect();
        for item in repeated {
            problems.push(Problem::new(format!(
                "T{n} has more than one acceptance item {item}"
            )));
        }
    }
    for link in &content.links {
        problems.extend(
            link_problems(content, removed, link)
                .into_iter()
                .map(Problem::new),
        );
    }
    if let Some(tasks) = cycle(content) {
        let names: Vec<String> = tasks.iter().map(|n| format!("T{n}")).collect();
        problems.push(Problem::new(format!(
            "these links form a cycle: {}",
            names.join(", ")
        )));
    }
    problems
}

fn link_problems(content: &PlanContent, removed: &BTreeSet<u32>, link: &Link) -> Vec<String> {
    // (the end that may be missing, the other end, how the first is linked to the other)
    let ends = [
        (link.after, link.task, "from"),
        (link.task, link.after, "to"),
    ];
    let ends = if link.task == link.after {
        &ends[..1]
    } else {
        &ends[..]
    };
    let missing: Vec<String> = ends
        .iter()
        .filter(|(end, _, _)| !content.tasks.contains_key(end))
        .map(|(end, other, direction)| {
            if removed.contains(end) {
                format!("T{end} is still linked {direction} T{other}")
            } else {
                format!("the link {direction} T{other} names T{end}, which does not exist")
            }
        })
        .collect();
    if !missing.is_empty() {
        return missing;
    }
    if link.task == link.after {
        return vec![format!("T{} cannot be linked to itself", link.task)];
    }
    let name = link_name(link.task, link.after, link.kind);
    match link.kind {
        LinkKind::Needs if !link.waiting_items.is_empty() => vec![format!(
            "{name} names acceptance items; a needs link waits for the whole task"
        )],
        LinkKind::Needs => vec![],
        LinkKind::CompletesAfter if link.waiting_items.is_empty() => {
            vec![format!("{name} names no acceptance item")]
        }
        LinkKind::CompletesAfter => {
            let waiting = &content.tasks[&link.task];
            link.waiting_items
                .iter()
                .filter(|&&n| !waiting.acceptance.iter().any(|item| item.number == n))
                .map(|n| format!("T{} has no acceptance item {n}", link.task))
                .collect()
        }
    }
}

/// §13.4: each task is two events, start and complete, start before complete.
/// `B needs A`: complete(A) → start(B). `B completes_after A`: complete(A) → complete(B).
/// A cycle is a strongly connected set of more than one event; its tasks are
/// named, never a task that merely comes after it. When several exist, the one
/// whose sorted task list is smallest is reported, so the message is stable.
fn cycle(content: &PlanContent) -> Option<Vec<u32>> {
    let mut g = DiGraph::<u32, ()>::new();
    let events: BTreeMap<u32, _> = content
        .tasks
        .keys()
        .map(|&n| {
            let s = g.add_node(n);
            let c = g.add_node(n);
            g.add_edge(s, c, ());
            (n, (s, c))
        })
        .collect();
    for l in &content.links {
        if l.task == l.after {
            continue; // reported as "cannot be linked to itself"
        }
        let (Some(&(b_start, b_complete)), Some(&(_, a_complete))) =
            (events.get(&l.task), events.get(&l.after))
        else {
            continue; // reported as a missing task
        };
        let to = match l.kind {
            LinkKind::Needs => b_start,
            LinkKind::CompletesAfter => b_complete,
        };
        g.add_edge(a_complete, to, ());
    }
    tarjan_scc(&g)
        .into_iter()
        .filter(|scc| scc.len() > 1)
        .map(|scc| {
            let mut t: Vec<u32> = scc.iter().map(|&i| g[i]).collect();
            t.sort_unstable();
            t.dedup();
            t
        })
        .min()
}

//! Start/complete ordering shared by local and cross-plan cycle checks.

use std::collections::BTreeMap;

use petgraph::{
    algo::tarjan_scc,
    graph::{DiGraph, NodeIndex},
};

use super::{Link, LinkKind, TaskParent};

pub(super) struct DependencyGraph {
    graph: DiGraph<(String, u32), ()>,
    events: BTreeMap<(String, u32), (NodeIndex, NodeIndex)>,
}

impl DependencyGraph {
    pub(super) fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            events: BTreeMap::new(),
        }
    }

    pub(super) fn task(&mut self, plan: &str, number: u32) {
        let key = (plan.to_string(), number);
        let start = self.graph.add_node(key.clone());
        let complete = self.graph.add_node(key.clone());
        self.graph.add_edge(start, complete, ());
        self.events.insert(key, (start, complete));
    }

    pub(super) fn link(&mut self, plan: &str, link: &Link) {
        let target = match &link.after {
            TaskParent::Local(number) => (plan.to_string(), *number),
            TaskParent::Plan { plan_id, task } => (plan_id.to_string(), *task),
        };
        let source = (plan.to_string(), link.task);
        if source == target {
            return;
        }
        let (Some(&(start, complete)), Some(&(_, prerequisite))) =
            (self.events.get(&source), self.events.get(&target))
        else {
            return;
        };
        let to = match link.kind {
            LinkKind::Needs => start,
            LinkKind::CompletesAfter => complete,
        };
        self.graph.add_edge(prerequisite, to, ());
    }

    pub(super) fn cycle(&self) -> Option<Vec<(String, u32)>> {
        tarjan_scc(&self.graph)
            .into_iter()
            .filter(|component| component.len() > 1)
            .map(|component| {
                let mut tasks: Vec<_> = component
                    .iter()
                    .map(|&node| self.graph[node].clone())
                    .collect();
                tasks.sort();
                tasks.dedup();
                tasks
            })
            .min()
    }
}

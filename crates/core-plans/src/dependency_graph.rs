//! Version-qualified start/complete ordering for tasks and phases.
use super::{Link, LinkKind, TaskContent, TaskParent};
use petgraph::{
    algo::tarjan_scc,
    graph::{DiGraph, NodeIndex},
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Key = (String, u32, Option<String>);
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct EventName {
    pub(super) plan: String,
    pub(super) task: u32,
    pub(super) phase: Option<u32>,
}
impl EventName {
    pub(super) fn task_label(&self) -> String {
        match self.phase {
            Some(phase) => format!("T{} · P{phase}", self.task),
            None => format!("T{}", self.task),
        }
    }
}

pub(super) struct DependencyGraph {
    graph: DiGraph<EventName, ()>,
    events: BTreeMap<Key, (NodeIndex, NodeIndex)>,
}
impl DependencyGraph {
    pub(super) fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            events: BTreeMap::new(),
        }
    }
    fn events(&mut self, key: Key, phase: Option<u32>) -> (NodeIndex, NodeIndex) {
        if let Some(events) = self.events.get(&key) {
            return *events;
        }
        let name = EventName {
            plan: key.0.clone(),
            task: key.1,
            phase,
        };
        let start = self.graph.add_node(name.clone());
        let complete = self.graph.add_node(name);
        self.graph.add_edge(start, complete, ());
        self.events.insert(key, (start, complete));
        (start, complete)
    }
    pub(super) fn phases(&mut self, plan: &str, task: &TaskContent) {
        let (start, complete) = self.events((plan.into(), task.number, None), None);
        let mut previous = start;
        for phase in &task.phases {
            let (begin, end) = self.events(
                (plan.into(), task.number, Some(phase.id.to_string())),
                Some(phase.number),
            );
            self.graph.add_edge(previous, begin, ());
            previous = end;
        }
        self.graph.add_edge(previous, complete, ());
    }
    pub(super) fn link(&mut self, plan: &str, link: &Link) {
        let target = match &link.after {
            TaskParent::Local(_) => plan,
            TaskParent::Plan { plan_id, .. } => plan_id.as_str(),
        };
        self.link_to(plan, target, link);
    }
    pub(super) fn link_to(&mut self, plan: &str, target: &str, link: &Link) {
        let source = (plan.into(), link.task, None);
        let target = (target.into(), link.after.number(), None);
        let (Some(&(start, complete)), Some(&(_, prerequisite))) =
            (self.events.get(&source), self.events.get(&target))
        else {
            return;
        };
        if source == target {
            return;
        }
        let to = if link.kind == LinkKind::Needs {
            start
        } else {
            complete
        };
        self.graph.add_edge(prerequisite, to, ());
    }
    pub(super) fn wait(&mut self, source: (&str, u32, &str), target: (&str, u32, &str)) {
        let (Some(&(start, _)), Some(&(_, complete))) = (
            self.events
                .get(&(source.0.into(), source.1, Some(source.2.into()))),
            self.events
                .get(&(target.0.into(), target.1, Some(target.2.into()))),
        ) else {
            return;
        };
        self.graph.add_edge(complete, start, ());
    }
    pub(super) fn cycle(&self) -> Option<Vec<EventName>> {
        let component = tarjan_scc(&self.graph)
            .into_iter()
            .filter(|c| c.len() > 1)
            .min_by_key(|c| c.iter().map(|n| (&self.graph[*n], n.index())).min())?;
        let allowed = component.iter().copied().collect::<BTreeSet<_>>();
        let start = *component
            .iter()
            .min_by_key(|n| (&self.graph[**n], n.index()))?;
        for first in self.neighbors(start, &allowed) {
            let mut pending = VecDeque::from([first]);
            let mut previous = BTreeMap::from([(first, None)]);
            while let Some(node) = pending.pop_front() {
                if node == start {
                    let mut path = vec![node];
                    let mut cursor = node;
                    while let Some(Some(parent)) = previous.get(&cursor) {
                        path.push(*parent);
                        cursor = *parent;
                    }
                    path.reverse();
                    path.insert(0, start);
                    let mut names = path
                        .into_iter()
                        .map(|n| self.graph[n].clone())
                        .collect::<Vec<_>>();
                    names.dedup();
                    if names.len() > 1 && names.first() == names.last() {
                        names.pop();
                    }
                    return Some(names);
                }
                for next in self.neighbors(node, &allowed) {
                    if let std::collections::btree_map::Entry::Vacant(entry) = previous.entry(next)
                    {
                        entry.insert(Some(node));
                        pending.push_back(next);
                    }
                }
            }
        }
        None
    }
    fn neighbors(&self, node: NodeIndex, allowed: &BTreeSet<NodeIndex>) -> Vec<NodeIndex> {
        let mut neighbors = self
            .graph
            .neighbors(node)
            .filter(|n| allowed.contains(n))
            .collect::<Vec<_>>();
        neighbors.sort_by_key(|n| (&self.graph[*n], n.index()));
        neighbors
    }
}

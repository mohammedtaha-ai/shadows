//! One job: resolving the uses of one plan version, from either end.
//!
//! A consumer stores only a reference to a producer's output; the producer's
//! own contract stays the one definition. A use joins the dependency graph
//! through the plan's existing links, never as a second kind of edge.

use crate::contract_questions::ConsumerRef;
use crate::dependencies::TaskParent;
use crate::model::PlanContent;
use crate::rules::Problem;
use crate::task_contract::{ContractRules, ContractSource, ContractTarget, ResolvedContract};

impl ContractRules {
    /// What blocks approval: a use whose producer, output or link is missing.
    /// Tasks ascending, each task's uses in their stored order.
    pub(super) fn approval_problems(content: &PlanContent) -> Vec<Problem> {
        let mut out = Vec::new();
        for (&n, task) in &content.tasks {
            let Some(contract) = &task.contract else {
                continue;
            };
            for u in &contract.uses {
                let ContractSource::Task {
                    target: ContractTarget::Local { task: p },
                    output: o,
                } = &u.source
                else {
                    continue;
                };
                let p = *p;
                let Some(producer) = content.tasks.get(&p) else {
                    out.push(Problem::new(format!(
                        "T{n} uses the output `{o}` of T{p}, which does not exist"
                    )));
                    continue;
                };
                let produced = producer
                    .contract
                    .as_ref()
                    .is_some_and(|c| c.produces.iter().any(|x| &x.id == o));
                if !produced {
                    out.push(Problem::new(format!(
                        "T{n} uses the output `{o}`, which T{p} does not produce"
                    )));
                    continue;
                }
                let linked = content
                    .links
                    .iter()
                    .any(|l| l.task == n && l.after == TaskParent::Local(p));
                if !linked {
                    out.push(Problem::new(format!(
                        "T{n} uses T{p}'s output `{o}` without a link to T{p}; add a needs link"
                    )));
                }
            }
        }
        out
    }

    /// The other tasks' uses whose source is exactly one of `sources`, by task
    /// number then use index: the planned consumers, never a guess by name.
    pub(super) fn consumers(
        content: &PlanContent,
        task: u32,
        sources: &[ContractSource],
    ) -> Vec<ConsumerRef> {
        let mut out = Vec::new();
        for (&n, other) in &content.tasks {
            let Some(contract) = other.contract.as_ref().filter(|_| n != task) else {
                continue;
            };
            for (i, u) in contract.uses.iter().enumerate() {
                if !sources.contains(&u.source) {
                    continue;
                }
                out.push(ConsumerRef {
                    task: n,
                    title: other.title.clone(),
                    output: match &u.source {
                        ContractSource::Task { output, .. } => Some(output.clone()),
                        ContractSource::Existing { .. } => None,
                    },
                    use_index: u32::try_from(i).unwrap_or(u32::MAX),
                    purpose: u.purpose.clone(),
                });
            }
        }
        out
    }

    /// The uses of `task`, in their stored order, each with what its producer
    /// promises. A use that does not resolve keeps its reference and has no
    /// title, symbol or artifact.
    pub(super) fn resolved(content: &PlanContent, task: u32) -> Vec<ResolvedContract> {
        let Some(contract) = content.tasks.get(&task).and_then(|t| t.contract.as_ref()) else {
            return Vec::new();
        };
        contract
            .uses
            .iter()
            .map(|u| {
                let mut resolved = ResolvedContract {
                    reference: u.source.clone(),
                    producer_title: None,
                    symbol: None,
                    artifact: None,
                };
                if let ContractSource::Task {
                    target: ContractTarget::Local { task: p },
                    output,
                } = &u.source
                    && let Some(producer) = content.tasks.get(p)
                    && let Some(made) = producer.contract.as_ref()
                    && let Some(out) = made.produces.iter().find(|x| &x.id == output)
                {
                    resolved.producer_title = Some(producer.title.clone());
                    resolved.symbol = out
                        .symbol_id
                        .as_ref()
                        .and_then(|id| made.symbols.iter().find(|s| &s.id == id))
                        .cloned();
                    resolved.artifact = out.artifact.clone();
                }
                resolved
            })
            .collect()
    }
}

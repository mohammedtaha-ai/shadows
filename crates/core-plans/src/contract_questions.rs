//! One job: answering what one task's contract promises, who produces what it
//! uses and which tasks of the same Frozen version plan to use it.
//!
//! Every answer comes from the one stored version: no newer version, no other
//! plan and no source code enters it. A consumer is a planned reference to a
//! use, never a place in source.
use crate::task_contract::{ContractRules, ContractSource, ContractTarget, ResolvedContract};
use crate::{Plans, PlansError, TaskContract, WorkflowId, WorkflowState};

/// Which part of a task's contract a question is about. Symbols and outputs
/// are the task's own; a use is one source the task declares.
#[derive(
    Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContractSelector {
    /// The whole contract of the task.
    #[default]
    Own,
    /// One symbol the task builds, by its id: `s1`.
    Symbol { symbol_id: String },
    /// One output the task produces, by its id: `o1`.
    Output { output_id: String },
    /// One source the task's contract uses, exactly as declared.
    Use { source: ContractSource },
}

/// A task of the same version that plans to use what was selected: a
/// reference to its use, never a source location.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ConsumerRef {
    pub task: u32,
    pub title: String,
    /// The producer's output the use names; null for a use of existing code.
    #[schema(required)]
    pub output: Option<String>,
    /// Where the use stands in the consumer's `uses`, from 0.
    pub use_index: u32,
    pub purpose: String,
}

/// What the plan's one Frozen version says about the selection.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractFacts {
    pub selector: ContractSelector,
    /// The selected part of the task's contract; null for a use, or a task
    /// without a contract.
    pub contract: Option<TaskContract>,
    pub resolved: Vec<ResolvedContract>,
    pub producers: Vec<ContractTarget>,
    /// One page, by task number then use index.
    pub consumers: Vec<ConsumerRef>,
    /// The `after` of the next page; null on the last.
    pub next: Option<u32>,
}

/// The most consumers one page answers.
const MOST: u16 = 50;

impl Plans {
    /// What task `task` of the Frozen version `workflow` promises for
    /// `selector`, with the page of planned consumers after the first
    /// `after`. `limit` is 1 to 50.
    pub async fn contract_facts(
        &self,
        workflow: &WorkflowId,
        task: u32,
        selector: ContractSelector,
        after: Option<u32>,
        limit: u16,
    ) -> Result<ContractFacts, PlansError> {
        let invalid = |m: String| PlansError::InvalidCommand(m);
        if !(1..=MOST).contains(&limit) {
            return Err(invalid(format!("limit is 1 to {MOST}, not {limit}")));
        }
        let plan = self.store.get_plan(workflow).await?;
        if plan.state != WorkflowState::Frozen {
            return Err(core_db::StorageError::NotFound("workflow").into());
        }
        let content = plan.content();
        let own = content
            .tasks
            .get(&task)
            .ok_or_else(|| PlansError::TaskNotInPlan(format!("T{task} is not in this plan")))?;
        let contract = own.contract.as_ref();
        let need = || invalid(format!("T{task} has no contract to select from"));
        let outputs = |ids: Vec<&String>| -> Vec<ContractSource> {
            ids.into_iter()
                .map(|id| ContractSource::Task {
                    target: ContractTarget::Local { task },
                    output: id.clone(),
                })
                .collect()
        };
        let (projection, resolved, producers, wanted) = match &selector {
            ContractSelector::Own => {
                let mut producers = Vec::new();
                for u in contract.map(|c| c.uses.as_slice()).unwrap_or_default() {
                    if let ContractSource::Task { target, .. } = &u.source
                        && !producers.contains(target)
                    {
                        producers.push(target.clone());
                    }
                }
                let ids = contract
                    .map(|c| c.produces.iter().map(|o| &o.id).collect())
                    .unwrap_or_default();
                let resolved = ContractRules::resolved(&content, task);
                (contract.cloned(), resolved, producers, outputs(ids))
            }
            ContractSelector::Symbol { symbol_id } => {
                let c = contract.ok_or_else(need)?;
                let symbol = (c.symbols.iter().find(|s| &s.id == symbol_id))
                    .ok_or_else(|| invalid(format!("T{task} has no symbol `{symbol_id}`")))?;
                let produces: Vec<_> = (c.produces.iter())
                    .filter(|o| o.symbol_id.as_ref() == Some(symbol_id))
                    .cloned()
                    .collect();
                let ids = produces.iter().map(|o| &o.id).collect();
                let wanted = outputs(ids);
                let projection = TaskContract {
                    files: Self::files_of(c, &[symbol.path.as_str()]),
                    symbols: vec![symbol.clone()],
                    produces,
                    ..Self::empty()
                };
                (Some(projection), Vec::new(), Vec::new(), wanted)
            }
            ContractSelector::Output { output_id } => {
                let c = contract.ok_or_else(need)?;
                let output = (c.produces.iter().find(|o| &o.id == output_id))
                    .ok_or_else(|| invalid(format!("T{task} has no output `{output_id}`")))?;
                let symbols: Vec<_> = (c.symbols.iter())
                    .filter(|s| output.symbol_id.as_ref() == Some(&s.id))
                    .cloned()
                    .collect();
                let mut paths: Vec<&str> = symbols.iter().map(|s| s.path.as_str()).collect();
                paths.extend(output.artifact.as_deref());
                let projection = TaskContract {
                    files: Self::files_of(c, &paths),
                    symbols,
                    produces: vec![output.clone()],
                    ..Self::empty()
                };
                let wanted = outputs(vec![&output.id]);
                (Some(projection), Vec::new(), Vec::new(), wanted)
            }
            ContractSelector::Use { source } => {
                let c = contract.ok_or_else(need)?;
                if !c.uses.iter().any(|u| &u.source == source) {
                    return Err(invalid(format!("T{task} declares no such use")));
                }
                let resolved = ContractRules::resolved(&content, task)
                    .into_iter()
                    .filter(|r| &r.reference == source)
                    .collect();
                let producers = match source {
                    ContractSource::Task { target, .. } => vec![target.clone()],
                    ContractSource::Existing { .. } => Vec::new(),
                };
                (None, resolved, producers, vec![source.clone()])
            }
        };
        let all = ContractRules::consumers(&content, task, &wanted);
        let from = after.unwrap_or(0) as usize;
        if from > all.len() {
            return Err(invalid(format!(
                "after {from} is past the {} planned consumers",
                all.len()
            )));
        }
        let to = (from + usize::from(limit)).min(all.len());
        let next = (to < all.len()).then(|| u32::try_from(to).unwrap_or(u32::MAX));
        Ok(ContractFacts {
            selector,
            contract: projection,
            resolved,
            producers,
            consumers: all[from..to].to_vec(),
            next,
        })
    }

    /// The task's file entries for `paths`, in the contract's order.
    fn files_of(c: &TaskContract, paths: &[&str]) -> Vec<crate::FileWork> {
        (c.files.iter())
            .filter(|f| paths.contains(&f.path.as_str()))
            .cloned()
            .collect()
    }

    fn empty() -> TaskContract {
        TaskContract {
            files: Vec::new(),
            symbols: Vec::new(),
            produces: Vec::new(),
            uses: Vec::new(),
            steps: Vec::new(),
            invariants: Vec::new(),
            forbidden: Vec::new(),
        }
    }
}

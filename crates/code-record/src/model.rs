//! One job: the code record's values, and the plain lines a verdict reads as.

use core_plans::{ElementAction, ElementPin, WorkflowId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Whether the code holds one promised symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SymbolState {
    Found,
    Missing,
    Unchecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BuildState {
    Planned,
    Built,
    Gap,
    Unchecked,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confirmation {
    FileAndSymbol,
    FileOnly,
}

/// A pin of a task that has not been marked done.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PlannedPin {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    pub pin: ElementPin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ElementTask {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    pub revision: i64,
    pub action: ElementAction,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_state: Option<SymbolState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ElementState {
    pub element_id: String,
    pub state: BuildState,
    pub built_revision: Option<i64>,
    pub behind: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed: Option<Confirmation>,
    pub done: Vec<ElementTask>,
    pub planned: Vec<ElementTask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// One symbol a task's contract promises, and what the code holds of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromisedSymbol {
    /// The contract's symbol id: `s1`.
    pub id: String,
    pub path: String,
    pub name: String,
    pub state: SymbolState,
    /// Why it is missing or unchecked; none when found.
    pub why: Option<String>,
}

/// A top-level symbol in a file the task creates that no contract symbol names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unpromised {
    pub path: String,
    pub name: String,
}

/// A file whose promised symbols could not all be checked, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UncheckedFile {
    pub path: String,
    pub why: String,
}

/// What `verify` found for one task's contract. Each list is ordered by path,
/// then by symbol id (`symbols`) or name (`unpromised`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    /// The task's number in its plan version, as the lines name it.
    pub task: u32,
    pub symbols: Vec<PromisedSymbol>,
    pub unpromised: Vec<Unpromised>,
    pub unchecked_files: Vec<UncheckedFile>,
}

/// A verdict as a check's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictOutcome {
    Passed,
    Failed,
    NotRun,
}

/// The `why` of every symbol of a promised file that is not in the folder.
pub(crate) const ABSENT: &str = "the file does not exist";

impl Verdict {
    /// Failed when a symbol is missing; passed when none is and one is found;
    /// not run when nothing could be checked. Unpromised symbols never fail it.
    pub fn outcome(&self) -> VerdictOutcome {
        let state = |s: SymbolState| self.symbols.iter().any(|p| p.state == s);
        if state(SymbolState::Missing) {
            VerdictOutcome::Failed
        } else if state(SymbolState::Found) {
            VerdictOutcome::Passed
        } else {
            VerdictOutcome::NotRun
        }
    }

    /// The check's output, one line each: missing symbols and absent files
    /// first, then the unchecked files, then the unpromised symbols.
    pub fn lines(&self) -> String {
        let mut out = Vec::new();
        let mut absent: Vec<&str> = Vec::new();
        for s in self
            .symbols
            .iter()
            .filter(|s| s.state == SymbolState::Missing)
        {
            if s.why.as_deref() == Some(ABSENT) {
                if !absent.contains(&s.path.as_str()) {
                    absent.push(&s.path);
                    out.push(format!("{} is promised and does not exist.", s.path));
                }
            } else {
                out.push(format!(
                    "T{} promises {} in {}; the code does not have it.",
                    self.task, s.name, s.path
                ));
            }
        }
        for f in &self.unchecked_files {
            out.push(format!("unchecked: {} ({}).", f.path, f.why));
        }
        for u in &self.unpromised {
            out.push(format!("unpromised: {} in {}.", u.name, u.path));
        }
        out.join("\n")
    }
}

/// What a done mark records of its task: the contract, and what the code held
/// of it. `attempt` and `commit` are none for a task done with no attempt;
/// `verdict` is none then, or when no verdict matches the recorded attempt,
/// and every promised symbol is recorded unchecked.
#[derive(Debug, Clone, PartialEq)]
pub struct DoneRecord {
    pub project: core_projects::ProjectId,
    pub workflow: core_plans::WorkflowId,
    pub version: i64,
    pub task: u32,
    pub attempt: Option<String>,
    pub commit: Option<String>,
    pub contract: core_plans::TaskContract,
    pub verdict: Option<Verdict>,
}

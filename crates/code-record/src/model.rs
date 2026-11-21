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
    /// Always sent: null while no done task built the element.
    #[schema(required)]
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
/// and every promised symbol is recorded unchecked. A task carried unchanged
/// from an earlier version names the attempt it was marked done on there.
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

/// How a `code_where` entry matched its topic, best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Rank {
    Exact,
    Path,
    Part,
    Words,
}

/// The answer to one question, record first.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WhereAnswer {
    pub topic: String,
    /// Ranked, at most 20.
    pub entries: Vec<WhereEntry>,
    pub entries_more: u32,
    pub not_in_record: Vec<UnrecordedFile>,
    pub not_in_record_more: u32,
    /// Language-server asks spent.
    pub asked: u32,
    /// Why the live half stopped early, when it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WhereEntry {
    File(FileEntry),
    Element(ElementEntry),
}

/// The part whose folder holds a path, and the first layer whose place does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct Owner {
    pub part_id: core_design::PartId,
    pub part_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FileEntry {
    pub rank: Rank,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Owner>,
    /// Newest first: version, then task, then workflow id.
    pub tasks: Vec<FileTask>,
    /// By name.
    pub symbols: Vec<SymbolEntry>,
    /// The file now.
    pub live: Live,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed: Option<Confirmation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct FileTask {
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    pub action: String,
    pub purpose: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SymbolEntry {
    pub symbol_id: String,
    pub name: String,
    pub kind: String,
    pub workflow_id: WorkflowId,
    pub version: i64,
    pub task: u32,
    /// From the recorded contract.
    pub behavior: Vec<String>,
    /// The contract's inputs, as the task wrote them.
    pub inputs: Vec<core_plans::ContractValue>,
    /// The contract's return; none for a class or when the contract gives none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns: Option<core_plans::ContractValue>,
    /// The state recorded when its task was marked done.
    pub at_done: SymbolState,
    pub live: Live,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// The reference lookup's outcome, apart from `live`.
    pub uses_live: UsesLive,
    /// Returned references, minus the symbol's own location, by path then line, at most 10.
    pub uses: Vec<UseLine>,
    /// Returned references beyond the 10 listed. A lower bound when `uses_live` is `truncated`;
    /// 0 when nothing was returned.
    pub uses_more: u32,
    /// The exact number of uses, given only when `uses_live` is `answered`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses_total: Option<u32>,
}

/// What the reference lookup of one symbol did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum UsesLive {
    NotAsked,
    Answered,
    Truncated { why: String },
    Failed { why: String },
    Skipped { why: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct UseLine {
    pub path: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Owner>,
}

/// What the code says now, for a file or a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Live {
    NotAsked,
    Confirmed,
    Gap { why: String },
    Unchecked { why: String },
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ElementEntry {
    pub rank: Rank,
    pub element: core_design::DesignElement,
    pub state: ElementState,
}

/// A file that matches the topic and that no done task's contract names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct UnrecordedFile {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Owner>,
    pub symbols: Vec<UnrecordedSymbol>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct UnrecordedSymbol {
    pub name: String,
    pub line: u32,
}

/// What the project's done tasks built, read from the record alone (no
/// language server), for a Planner's context.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordDigest {
    /// The parts of the approved map that hold recorded files, in map order.
    pub parts: Vec<DigestPart>,
    /// Recorded files no part's folder holds, by path.
    pub unowned: Vec<DigestFile>,
    /// The elements in gap or unchecked, by their part's map order, then element number.
    pub gaps: Vec<DigestGap>,
    /// The count of done marks recorded: tasks with at least one record row.
    pub marks: u32,
}

impl RecordDigest {
    /// True when no done task recorded a file and no element is in gap or unchecked.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty() && self.unowned.is_empty() && self.gaps.is_empty()
    }
}

/// One part's recorded files, by layer in the part's order (no layer last), then path.
#[derive(Debug, Clone, PartialEq)]
pub struct DigestPart {
    pub part_id: core_design::PartId,
    pub title: String,
    pub files: Vec<DigestFile>,
}

/// A recorded file: the purpose of its newest record, and the newest record of
/// each of its symbols, by name.
#[derive(Debug, Clone, PartialEq)]
pub struct DigestFile {
    pub path: String,
    /// The first layer of its part whose place holds it.
    pub layer: Option<String>,
    pub purpose: String,
    pub symbols: Vec<DigestSymbol>,
}

/// A recorded symbol and its signature, as its contract wrote it.
#[derive(Debug, Clone, PartialEq)]
pub struct DigestSymbol {
    pub name: String,
    pub kind: core_plans::SymbolKind,
    pub inputs: Vec<core_plans::ContractValue>,
    pub returns: Option<core_plans::ContractValue>,
}

/// An element in gap or unchecked, with why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestGap {
    pub element_id: String,
    pub kind: core_design::ElementKind,
    pub name: String,
    pub part_id: core_design::PartId,
    pub state: BuildState,
    pub why: String,
}

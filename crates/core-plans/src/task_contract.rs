//! One job: the task contract types, with the rules one task's contract must hold.
//!
//! A contract names a task's work exactly: the files it reads, creates,
//! modifies or deletes, and the behavior of each function, method or class it
//! builds. It never carries code. `reads` and `writes` stay the task's
//! permission scope; a contract is not permission to widen them.

use std::collections::BTreeSet;

use crate::model::PlanContent;
use crate::rules::Problem;

/// What a task builds, named exactly: the executor's brief beside the goal.
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct TaskContract {
    #[serde(default)]
    pub files: Vec<FileWork>,
    #[serde(default)]
    pub symbols: Vec<SymbolWork>,
    /// The order of work, when it matters.
    #[serde(default)]
    pub steps: Vec<String>,
    /// What must stay true.
    #[serde(default)]
    pub invariants: Vec<String>,
    /// What must not be changed.
    #[serde(default)]
    pub forbidden: Vec<String>,
}

/// One file the task works on, and why.
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct FileWork {
    /// One exact file, relative to the project's folder, with '/'. Never a pattern.
    pub path: String,
    pub action: FileAction,
    pub purpose: String,
}

/// What the task does to a file.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Read,
    Create,
    Modify,
    Delete,
}

/// One function, method or class the task builds, described by its behavior.
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct SymbolWork {
    /// Chosen by the writer, unique within the task: `s1`.
    pub id: String,
    /// One of the task's files that it creates or modifies.
    pub path: String,
    pub kind: SymbolKind,
    pub name: String,
    #[serde(default)]
    pub inputs: Vec<ContractValue>,
    /// What it answers; null for a class.
    #[serde(default)]
    pub returns: Option<ContractValue>,
    /// Short sentences saying what it does, never its code.
    pub behavior: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}

/// What kind of symbol a task builds.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
}

/// A parameter or a return value.
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ContractValue {
    /// The parameter's name; empty for a return value.
    #[serde(default)]
    pub name: String,
    /// As the project's language writes the type.
    pub type_name: String,
    pub meaning: String,
}

/// The rules every stored contract holds.
pub(super) struct ContractRules;

impl ContractRules {
    /// What every stored contract must hold, tasks ascending, each task's
    /// problems in the order of the list below.
    pub(super) fn problems(content: &PlanContent) -> Vec<Problem> {
        let mut out = Vec::new();
        for (n, task) in &content.tasks {
            if let Some(contract) = &task.contract {
                Self::task(*n, contract, &mut out);
            }
        }
        out
    }

    fn task(n: u32, c: &TaskContract, out: &mut Vec<Problem>) {
        let mut say = |m: String| out.push(Problem::new(format!("T{n} contract: {m}")));
        if c.files.is_empty() && c.symbols.is_empty() {
            say("it names no file".into());
        }
        for f in &c.files {
            if !Self::exact(&f.path) {
                say(format!(
                    "`{}` is not an exact path inside the project",
                    f.path
                ));
            }
        }
        let mut seen = BTreeSet::new();
        let mut twice = BTreeSet::new();
        for f in &c.files {
            if !seen.insert(f.path.as_str()) && twice.insert(f.path.as_str()) {
                say(format!("the file `{}` is named more than once", f.path));
            }
        }
        for f in &c.files {
            if f.purpose.trim().is_empty() {
                say(format!("the file `{}` has no purpose", f.path));
            }
        }
        let mut ids = BTreeSet::new();
        let mut said = BTreeSet::new();
        for s in &c.symbols {
            let fresh = ids.insert(s.id.as_str());
            if (s.id.trim().is_empty() || !fresh) && said.insert(s.id.as_str()) {
                say(format!("missing or repeated symbol id `{}`", s.id));
            }
        }
        let built: BTreeSet<&str> = c
            .files
            .iter()
            .filter(|f| matches!(f.action, FileAction::Create | FileAction::Modify))
            .map(|f| f.path.as_str())
            .collect();
        let mut described = BTreeSet::new();
        for s in &c.symbols {
            let id = &s.id;
            if s.name.trim().is_empty() {
                say(format!("the symbol `{id}` has no name"));
            }
            if !built.contains(s.path.as_str()) {
                say(format!(
                    "the symbol `{id}` is in `{}`, which the task does not create or modify",
                    s.path
                ));
            }
            if !described.insert((s.path.as_str(), s.kind as u8, s.name.as_str())) {
                say(format!(
                    "`{}` is described more than once in `{}`",
                    s.name, s.path
                ));
            }
            let blank_input = s
                .inputs
                .iter()
                .any(|i| i.name.trim().is_empty() || i.type_name.trim().is_empty());
            if blank_input {
                say(format!("an input of `{id}` needs a name and a type"));
            }
            let returns = s
                .returns
                .as_ref()
                .is_some_and(|r| !r.type_name.trim().is_empty());
            if s.kind != SymbolKind::Class && !returns {
                say(format!("`{id}` needs a return type"));
            }
            if s.behavior.is_empty() || s.behavior.iter().any(|b| b.trim().is_empty()) {
                say(format!("`{id}` says no behavior"));
            }
        }
        for (field, lines) in [
            ("steps", &c.steps),
            ("invariants", &c.invariants),
            ("forbidden", &c.forbidden),
        ] {
            if lines.iter().any(|l| l.trim().is_empty()) {
                say(format!("a blank line in {field}"));
            }
        }
    }

    /// One exact relative path inside the project: no pattern, no way out.
    fn exact(path: &str) -> bool {
        !path.is_empty()
            && !path.starts_with('/')
            && !path.ends_with('/')
            && !path.contains(['\\', ':', '*', '?', '['])
            && !path.split('/').any(|c| c == "..")
    }
}

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
    #[serde(default)]
    pub produces: Vec<ProducedOutput>,
    #[serde(default)]
    pub uses: Vec<ContractUse>,
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

/// Something other tasks may rely on: one symbol or one file of this task.
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
pub struct ProducedOutput {
    /// Chosen by the writer, unique within the task: `o1`.
    pub id: String,
    #[serde(default)]
    pub symbol_id: Option<String>,
    /// A file path this task creates or modifies.
    #[serde(default)]
    pub artifact: Option<String>,
    /// The task's acceptance items that prove it.
    #[serde(default)]
    pub acceptance: Vec<u32>,
}

/// Something this task relies on, and why.
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
pub struct ContractUse {
    pub source: ContractSource,
    pub purpose: String,
}

/// Where a used output comes from.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContractSource {
    /// An output another task of this plan version produces.
    Task {
        target: ContractTarget,
        output: String,
    },
    /// Code the project already holds. `symbol` is empty for the whole file.
    Existing {
        path: String,
        #[serde(default)]
        symbol: String,
    },
}

/// The task that produces a used output.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    schemars::JsonSchema,
)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContractTarget {
    Local { task: u32 },
}

/// A use, with what its producer promises. The consumer never stores this.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ResolvedContract {
    pub reference: ContractSource,
    pub producer_title: Option<String>,
    pub symbol: Option<SymbolWork>,
    pub artifact: Option<String>,
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
                let acceptance: BTreeSet<u32> = task.acceptance.iter().map(|a| a.number).collect();
                Self::task(*n, contract, &acceptance, &mut out);
            }
        }
        out
    }

    fn task(n: u32, c: &TaskContract, acceptance: &BTreeSet<u32>, out: &mut Vec<Problem>) {
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
        let symbols: BTreeSet<&str> = c.symbols.iter().map(|s| s.id.as_str()).collect();
        let mut outputs = BTreeSet::new();
        let mut said = BTreeSet::new();
        for o in &c.produces {
            let fresh = outputs.insert(o.id.as_str());
            if (o.id.trim().is_empty() || !fresh) && said.insert(o.id.as_str()) {
                say(format!("missing or repeated output id `{}`", o.id));
            }
        }
        for o in &c.produces {
            let id = &o.id;
            if o.symbol_id.is_some() == o.artifact.is_some() {
                say(format!(
                    "the output `{id}` names exactly one of a symbol or an artifact"
                ));
            }
            if let Some(symbol) = &o.symbol_id
                && !symbols.contains(symbol.as_str())
            {
                say(format!(
                    "the output `{id}` names the symbol `{symbol}`, \
                     which the task does not describe"
                ));
            }
            if let Some(path) = &o.artifact
                && !built.contains(path.as_str())
            {
                say(format!(
                    "the output `{id}` names `{path}`, which the task does not create or modify"
                ));
            }
            for k in &o.acceptance {
                if !acceptance.contains(k) {
                    say(format!(
                        "the output `{id}` names acceptance item {k}, which T{n} does not have"
                    ));
                }
            }
        }
        let mut sources: Vec<&ContractSource> = Vec::new();
        let mut repeated = false;
        for u in &c.uses {
            if u.purpose.trim().is_empty() {
                say("a use has no purpose".into());
            }
            match &u.source {
                ContractSource::Existing { path, .. } if !Self::exact(path) => {
                    say(format!("`{path}` is not an exact path inside the project"));
                }
                ContractSource::Task {
                    target: ContractTarget::Local { task },
                    ..
                } if *task == n => say("a task cannot use its own output".into()),
                _ => {}
            }
            if sources.contains(&&u.source) {
                repeated = true;
            }
            sources.push(&u.source);
        }
        if repeated {
            say("the same source is used more than once".into());
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

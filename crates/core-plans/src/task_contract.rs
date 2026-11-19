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
    /// Commands that prove acceptance, run by the executor and again by Shadows.
    #[serde(default)]
    pub checks: Vec<TaskCheck>,
    /// Accepted project decisions the task follows, each at one exact revision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(required = false)]
    pub decisions: Vec<crate::DecisionPin>,
}

/// A program with arguments that proves acceptance items without a shell.
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
pub struct TaskCheck {
    /// Chosen by the writer, unique within the task: `c1`.
    pub id: String,
    /// The program, then its arguments, never a shell line:
    /// `["go", "test", "./internal/api/..."]`.
    pub run: Vec<String>,
    /// Where it runs, relative to the project's folder; the folder itself when left out.
    #[serde(default)]
    pub folder: Option<String>,
    /// Seconds before it is stopped: 1 to 1800, 600 when left out.
    #[serde(default)]
    pub timeout_secs: Option<u32>,
    /// The task's acceptance items it proves.
    #[serde(default)]
    pub proves: Vec<u32>,
}

impl TaskCheck {
    pub const DEFAULT_TIMEOUT_SECS: u32 = 600;
    pub const MAX_TIMEOUT_SECS: u32 = 1800;

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(u64::from(
            self.timeout_secs.unwrap_or(Self::DEFAULT_TIMEOUT_SECS),
        ))
    }
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
    /// As the code writes it: a bare name (`refresh_token`), or `Class.method`
    /// for a method of one class.
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
    /// Every path a contract in `tasks` would create, modify or delete: what
    /// the project's file list is asked about before an edit is saved.
    pub(super) fn worked_paths<'a>(
        tasks: impl IntoIterator<Item = &'a crate::TaskContent>,
    ) -> Vec<String> {
        let mut paths = BTreeSet::new();
        for task in tasks {
            for f in task.contract.iter().flat_map(|c| &c.files) {
                if matches!(
                    f.action,
                    FileAction::Create | FileAction::Modify | FileAction::Delete
                ) {
                    paths.insert(f.path.clone());
                }
            }
        }
        paths.into_iter().collect()
    }

    /// The files a task in `checked` works on must agree with the project's
    /// file list (`present`) and with the tasks it depends on, directly or
    /// through others. A file it modifies or deletes must be present or
    /// created by such a task, and never deleted by one. A file it creates
    /// must not be present, unless such a task deletes it first, nor created
    /// by such a task; a create the task already had before this edit
    /// (`before`) is not checked again, since the task may have run and made
    /// it. One problem per file, tasks ascending.
    pub(super) fn file_problems(
        before: &PlanContent,
        content: &PlanContent,
        checked: &[u32],
        present: &std::collections::HashSet<String>,
    ) -> Vec<Problem> {
        let mut out = Vec::new();
        let numbers: BTreeSet<u32> = checked.iter().copied().collect();
        for n in numbers {
            let Some(contract) = content.tasks.get(&n).and_then(|t| t.contract.as_ref()) else {
                continue;
            };
            let ancestors = Self::ancestors(content, n);
            let along = |action: FileAction, path: &str| {
                ancestors.iter().any(|a| {
                    content.tasks[a]
                        .contract
                        .iter()
                        .flat_map(|c| &c.files)
                        .any(|f| f.action == action && f.path == path)
                })
            };
            let had_create = |path: &str| {
                before
                    .tasks
                    .get(&n)
                    .and_then(|t| t.contract.as_ref())
                    .is_some_and(|c| {
                        c.files
                            .iter()
                            .any(|f| f.action == FileAction::Create && f.path == path)
                    })
            };
            for f in &contract.files {
                let path = &f.path;
                match f.action {
                    FileAction::Modify | FileAction::Delete => {
                        let verb = if f.action == FileAction::Modify {
                            "modifies"
                        } else {
                            "deletes"
                        };
                        if along(FileAction::Delete, path) {
                            out.push(Problem::new(format!(
                                "T{n} {verb} `{path}`, which a task it depends on deletes"
                            )));
                        } else if !present.contains(path) && !along(FileAction::Create, path) {
                            out.push(Problem::new(format!(
                                "T{n} {verb} `{path}`, which the project does not have and no task it depends on creates"
                            )));
                        }
                    }
                    FileAction::Create if !had_create(path) => {
                        if along(FileAction::Create, path) {
                            out.push(Problem::new(format!(
                                "T{n} creates `{path}`, which a task it depends on already creates: modify it instead"
                            )));
                        } else if present.contains(path) && !along(FileAction::Delete, path) {
                            out.push(Problem::new(format!(
                                "T{n} creates `{path}`, which the project already has: modify it instead, and ask Shadows (code_lsp) what it holds"
                            )));
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// The tasks `n` depends on through the plan's links, directly or through others.
    fn ancestors(content: &PlanContent, n: u32) -> BTreeSet<u32> {
        let mut found = BTreeSet::new();
        let mut next = vec![n];
        while let Some(task) = next.pop() {
            for link in content.links.iter().filter(|l| l.task == task) {
                if let Some(after) = link.after.local()
                    && content.tasks.contains_key(&after)
                    && after != n
                    && found.insert(after)
                {
                    next.push(after);
                }
            }
        }
        found
    }
}

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
        let mut checks = BTreeSet::new();
        for check in &c.checks {
            let id = &check.id;
            if id.trim().is_empty() || !checks.insert(id) {
                say(format!("missing or repeated check id `{id}`"));
            }
            if check
                .run
                .first()
                .is_none_or(|program| program.trim().is_empty())
            {
                say(format!("the check `{id}` names no program"));
            }
            if let Some(folder) = &check.folder
                && folder != "."
                && !Self::exact(folder)
            {
                say(format!(
                    "the check `{id}` runs in `{folder}`, which is not a folder inside the project"
                ));
            }
            if check
                .timeout_secs
                .is_some_and(|t| t == 0 || t > TaskCheck::MAX_TIMEOUT_SECS)
            {
                say(format!(
                    "the check `{id}` has a time limit outside 1 to 1800 seconds"
                ));
            }
            for k in &check.proves {
                if !acceptance.contains(k) {
                    say(format!(
                        "the check `{id}` proves acceptance item {k}, which T{n} does not have"
                    ));
                }
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

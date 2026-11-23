//! One job: comparing what a task's contract promised with the symbols the
//! project's language servers find in the folder as it stands.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use codebase::{Asker, LspAnswer, LspOperation, LspQuestion, ServerState};
use core_plans::{FileAction, TaskContract};
use core_projects::ProjectId;
use serde_json::Value;

use crate::model::ABSENT;
use crate::{
    CodeRecord, CodeRecordError, PromisedSymbol, SymbolState, UncheckedFile, Unpromised, Verdict,
};

/// How often a server that would read the file but is not running yet is asked again.
const RETRY_EVERY: Duration = Duration::from_secs(5);
/// The longest a file waits for such a server.
const RETRY_FOR: Duration = Duration::from_secs(120);

/// The `documentSymbol` kinds that count: class, method, constructor, enum,
/// interface, function and struct. Enum, interface and struct are the
/// languages' type-like kinds and match a contract's `class`. Field, variable,
/// constant, property and module never count.
const COUNTED: [u64; 7] = [5, 6, 9, 10, 11, 12, 23];
/// Kind Object, which rust-analyzer gives an `impl` block.
const IMPL: u64 = 19;

/// One counted symbol of a `documentSymbol` answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Found {
    pub(crate) name: String,
    /// The counted symbol it is inside, if any.
    pub(crate) parent: Option<String>,
    pub(crate) top: bool,
    /// One-based, as Codebase's rewrite gives it; none in a raw answer without it.
    pub(crate) line: Option<u32>,
    pub(crate) character: Option<u32>,
}

/// What one file's question came back with.
#[derive(Debug)]
pub(crate) enum FileAnswer {
    Absent,
    Symbols { found: Vec<Found>, truncated: bool },
    Unchecked(String),
}

impl CodeRecord {
    /// Compares `contract`'s promised symbols with the code, stores the verdict
    /// under `key` (an attempt id, or a recheck's key), replacing one stored
    /// before, and answers it. Each distinct file is asked once: whether it
    /// exists, then `documentSymbol` as a system call made for the project.
    /// Only storage fails it.
    pub async fn verify(
        &self,
        key: &str,
        project: &ProjectId,
        task: u32,
        contract: &TaskContract,
    ) -> Result<Verdict, CodeRecordError> {
        let created: BTreeSet<&str> = contract
            .files
            .iter()
            .filter(|f| f.action == FileAction::Create)
            .map(|f| f.path.as_str())
            .collect();
        let paths: BTreeSet<&str> = contract
            .symbols
            .iter()
            .map(|s| s.path.as_str())
            .chain(created.iter().copied())
            .collect();
        let mut answers = BTreeMap::new();
        for path in paths {
            answers.insert(path.to_string(), self.ask(project, path).await);
        }
        let verdict = CodeRecord::compare(task, contract, &answers);
        self.store.put(key, project, &verdict).await?;
        Ok(verdict)
    }

    /// One file: absent, its counted symbols, or why it could not be checked.
    async fn ask(&self, project: &ProjectId, path: &str) -> FileAnswer {
        #[cfg(feature = "test-support")]
        {
            use crate::RecordedAnswer;
            match self.recorded(path) {
                Some(RecordedAnswer::Symbols(result)) => {
                    return CodeRecord::file_answer(&result, false);
                }
                Some(RecordedAnswer::NoServer) => {
                    return FileAnswer::Unchecked("no language server reads it".into());
                }
                Some(RecordedAnswer::Absent) => return FileAnswer::Absent,
                Some(RecordedAnswer::Failed(why)) => return FileAnswer::Unchecked(why),
                Some(RecordedAnswer::References { .. } | RecordedAnswer::Workspace(_)) | None => {}
            }
        }
        match self
            .code
            .missing_inputs(Asker::Person(project), &[path.to_string()])
            .await
        {
            Ok(absent) if !absent.is_empty() => return FileAnswer::Absent,
            Ok(_) => {}
            Err(error) => return FileAnswer::Unchecked(format!("it could not be read: {error}")),
        }
        let started = Instant::now();
        loop {
            let question = LspQuestion {
                operation: LspOperation::DocumentSymbol,
                file: Some(path.to_string()),
                line: None,
                character: None,
                query: None,
                project: None,
            };
            let answer = match self.code.lsp(Asker::Person(project), question).await {
                Ok(answer) => answer,
                Err(error) => return FileAnswer::Unchecked(error.to_string()),
            };
            match answer {
                LspAnswer::Answered {
                    result, truncated, ..
                } => return CodeRecord::file_answer(&result, truncated),
                LspAnswer::NoServer {
                    server: Some(server),
                    state: None | Some(ServerState::Idle | ServerState::Starting),
                    ..
                } => {
                    if started.elapsed() >= RETRY_FOR {
                        return FileAnswer::Unchecked(format!(
                            "the language server {server} did not start in 2 minutes"
                        ));
                    }
                    tokio::time::sleep(RETRY_EVERY).await;
                }
                LspAnswer::NoServer {
                    server: Some(server),
                    state,
                    ..
                } => {
                    let why = match state {
                        Some(ServerState::NotInstalled) => "is not installed".to_string(),
                        Some(ServerState::Off) => "is switched off".to_string(),
                        Some(ServerState::Failed { error: Some(e) }) => format!("failed: {e}"),
                        _ => "failed".to_string(),
                    };
                    return FileAnswer::Unchecked(format!("the language server {server} {why}"));
                }
                LspAnswer::NoServer { server: None, .. } => {
                    return FileAnswer::Unchecked("no language server reads it".into());
                }
            }
        }
    }

    /// The answer a test recorded under `key`, if any.
    #[cfg(feature = "test-support")]
    pub(crate) fn recorded(&self, key: &str) -> Option<crate::RecordedAnswer> {
        self.recorded
            .lock()
            .expect("recorded answers")
            .get(key)
            .cloned()
    }

    /// A `documentSymbol` result as one file's answer.
    pub(crate) fn file_answer(result: &Value, truncated: bool) -> FileAnswer {
        FileAnswer::Symbols {
            found: CodeRecord::document_symbols(result),
            truncated,
        }
    }

    /// The counted symbols of a `documentSymbol` result, at any depth: a
    /// `DocumentSymbol` tree (`children`) or a flat `SymbolInformation` list
    /// (`containerName`). A variable, parameter, field or module is never
    /// counted. A method's parent is its type, which `owner` reads from an
    /// `impl` block and `receiver` from a gopls `(*X).m`; either makes it not
    /// top-level, and an `impl` block is no symbol of its own.
    pub(crate) fn document_symbols(result: &Value) -> Vec<Found> {
        fn walk(items: &Value, parent: Option<&str>, top: bool, out: &mut Vec<Found>) {
            for item in items.as_array().into_iter().flatten() {
                let name = item.get("name").and_then(Value::as_str);
                let kind = item.get("kind").and_then(Value::as_u64);
                let flat_parent = item.get("containerName").and_then(Value::as_str);
                let (parent, top) = match flat_parent {
                    Some(c) if !c.is_empty() => (Some(CodeRecord::owner(c)), false),
                    _ => (parent, top),
                };
                let impl_of = name
                    .filter(|_| kind == Some(IMPL))
                    .map(CodeRecord::owner)
                    .filter(|owner| Some(*owner) != name);
                let counted = impl_of.is_none() && kind.is_some_and(|k| COUNTED.contains(&k));
                let (name, parent, top) = match name.and_then(CodeRecord::receiver) {
                    Some((method, receiver)) => (Some(method), Some(receiver), false),
                    None => (name, parent, top),
                };
                if let (Some(name), true) = (name, counted) {
                    let at = |key: &str| {
                        item.get(key)
                            .and_then(Value::as_u64)
                            .and_then(|n| u32::try_from(n).ok())
                    };
                    out.push(Found {
                        name: name.to_string(),
                        parent: parent.map(str::to_string),
                        top,
                        line: at("line"),
                        character: at("character"),
                    });
                }
                if let Some(children) = item.get("children") {
                    let next = match impl_of {
                        Some(owner) => Some(owner),
                        None if counted => name,
                        None => parent,
                    };
                    walk(children, next, false, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(result, None, true, &mut out);
        out
    }

    /// The type a container stands for. rust-analyzer names an `impl` block
    /// (`impl<T> Trait for path::X<T>`) rather than its type: the type is the
    /// part after `for` (else after `impl` and its `<...>`), without its
    /// generic arguments, at its last path segment. Any other name is itself.
    fn owner(container: &str) -> &str {
        let Some(rest) = container.strip_prefix("impl") else {
            return container;
        };
        if !rest.starts_with([' ', '<']) {
            return container;
        }
        let mut ty = rest.trim_start();
        let mut depth = 0usize;
        let mut generics_end = None;
        for (i, c) in ty.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if i == 0 && c != '<' {
                break;
            }
            if depth == 0 {
                generics_end = Some(i + 1);
                break;
            }
        }
        if let Some(end) = generics_end {
            ty = &ty[end..];
        }
        let mut depth = 0usize;
        for (i, c) in ty.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => depth = depth.saturating_sub(1),
                ' ' if depth == 0 && ty[i..].starts_with(" for ") => {
                    ty = &ty[i + " for ".len()..];
                    break;
                }
                _ => {}
            }
        }
        let ty = ty.trim();
        let ty = ty.split_once('<').map_or(ty, |(head, _)| head).trim_end();
        ty.rsplit("::").next().unwrap_or(ty)
    }

    /// A gopls method name, `(*X).m` or `(X).m`, as its method and its type.
    fn receiver(name: &str) -> Option<(&str, &str)> {
        let (receiver, method) = name.strip_prefix('(')?.split_once(").")?;
        let receiver = receiver.trim_start_matches('*');
        let receiver = receiver.split_once('[').map_or(receiver, |(head, _)| head);
        Some((method, receiver))
    }

    /// Whether a contract name matches a counted symbol: exactly, at any
    /// depth, or `Class.method` as `method` inside `Class`.
    pub(crate) fn matches(name: &str, found: &Found) -> bool {
        match name.rsplit_once('.') {
            Some((class, method)) => found.name == method && found.parent.as_deref() == Some(class),
            None => found.name == name,
        }
    }

    /// The verdict over each file's answer.
    fn compare(
        task: u32,
        contract: &TaskContract,
        answers: &BTreeMap<String, FileAnswer>,
    ) -> Verdict {
        let mut symbols = Vec::new();
        let mut unchecked = BTreeMap::new();
        for s in &contract.symbols {
            let (state, why) = match answers.get(&s.path) {
                Some(FileAnswer::Absent) => (SymbolState::Missing, Some(ABSENT.to_string())),
                Some(FileAnswer::Symbols { found, truncated }) => {
                    if found.iter().any(|f| CodeRecord::matches(&s.name, f)) {
                        (SymbolState::Found, None)
                    } else if *truncated {
                        let why = "the language server's answer was cut short".to_string();
                        unchecked.insert(s.path.clone(), why.clone());
                        (SymbolState::Unchecked, Some(why))
                    } else {
                        (
                            SymbolState::Missing,
                            Some("the code does not have it".to_string()),
                        )
                    }
                }
                Some(FileAnswer::Unchecked(why)) => {
                    unchecked.insert(s.path.clone(), why.clone());
                    (SymbolState::Unchecked, Some(why.clone()))
                }
                None => {
                    let why = "it was not asked about".to_string();
                    unchecked.insert(s.path.clone(), why.clone());
                    (SymbolState::Unchecked, Some(why))
                }
            };
            symbols.push(PromisedSymbol {
                id: s.id.clone(),
                path: s.path.clone(),
                name: s.name.clone(),
                state,
                why,
            });
        }
        symbols.sort_by(|a, b| (&a.path, &a.id).cmp(&(&b.path, &b.id)));
        let mut unpromised = Vec::new();
        for f in contract
            .files
            .iter()
            .filter(|f| f.action == FileAction::Create)
        {
            let Some(FileAnswer::Symbols { found, .. }) = answers.get(&f.path) else {
                continue;
            };
            let named = |name: &str| {
                contract.symbols.iter().any(|s| {
                    s.path == f.path
                        && (s.name == name
                            || s.name
                                .rsplit_once('.')
                                .is_some_and(|(class, _)| class == name))
                })
            };
            for top in found.iter().filter(|x| x.top && !named(&x.name)) {
                unpromised.push(Unpromised {
                    path: f.path.clone(),
                    name: top.name.clone(),
                });
            }
        }
        unpromised.sort_by(|a, b| (&a.path, &a.name).cmp(&(&b.path, &b.name)));
        unpromised.dedup();
        Verdict {
            task,
            symbols,
            unpromised,
            unchecked_files: unchecked
                .into_iter()
                .map(|(path, why)| UncheckedFile { path, why })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FileAnswer;
    use crate::{CodeRecord, SymbolState, Unpromised};
    use core_plans::{ContractValue, FileWork, SymbolKind, SymbolWork};
    use core_plans::{FileAction, TaskContract};
    use serde_json::Value;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn contract(files: &[(&str, FileAction)], symbols: &[(&str, &str, &str)]) -> TaskContract {
        TaskContract {
            files: files
                .iter()
                .map(|(path, action)| FileWork {
                    path: path.to_string(),
                    action: *action,
                    purpose: "p".into(),
                })
                .collect(),
            symbols: symbols
                .iter()
                .map(|(id, path, name)| SymbolWork {
                    id: id.to_string(),
                    path: path.to_string(),
                    kind: SymbolKind::Function,
                    name: name.to_string(),
                    inputs: Vec::new(),
                    returns: Some(ContractValue {
                        name: String::new(),
                        type_name: "None".into(),
                        meaning: "m".into(),
                    }),
                    behavior: vec!["b".into()],
                    errors: Vec::new(),
                })
                .collect(),
            produces: Vec::new(),
            uses: Vec::new(),
            steps: Vec::new(),
            invariants: Vec::new(),
            forbidden: Vec::new(),
            checks: Vec::new(),
            decisions: Vec::new(),
            elements: Vec::new(),
        }
    }

    /// What pyright answers for a module with a class, its method and field,
    /// a function with a parameter, and a module variable.
    fn pyright_answer() -> Value {
        json!([
            { "name": "Auth", "kind": 5, "children": [
                { "name": "__init__", "kind": 6, "children": [
                    { "name": "self", "kind": 13 }
                ]},
                { "name": "refresh", "kind": 6 },
                { "name": "token", "kind": 8 }
            ]},
            { "name": "login", "kind": 12, "children": [
                { "name": "user", "kind": 13 }
            ]},
            { "name": "LIMIT", "kind": 13 }
        ])
    }

    #[test]
    fn document_symbols_count_classes_methods_and_functions_only() {
        let found = CodeRecord::document_symbols(&pyright_answer());
        let names: Vec<(&str, Option<&str>, bool)> = found
            .iter()
            .map(|f| (f.name.as_str(), f.parent.as_deref(), f.top))
            .collect();
        assert_eq!(
            names,
            vec![
                ("Auth", None, true),
                ("__init__", Some("Auth"), false),
                ("refresh", Some("Auth"), false),
                ("login", None, true),
            ]
        );
        let flat = json!([
            { "name": "Auth", "kind": 5, "location": {} },
            { "name": "refresh", "kind": 6, "containerName": "Auth", "location": {} },
            { "name": "LIMIT", "kind": 14, "location": {} }
        ]);
        let found = CodeRecord::document_symbols(&flat);
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].parent.as_deref(), Some("Auth"));
        assert!(!found[1].top);
    }

    #[test]
    fn a_truncated_answer_never_makes_a_symbol_missing() {
        let c = contract(
            &[("a.py", FileAction::Modify)],
            &[("s1", "a.py", "login"), ("s2", "a.py", "logout")],
        );
        let answers = BTreeMap::from([(
            "a.py".to_string(),
            FileAnswer::Symbols {
                found: CodeRecord::document_symbols(&pyright_answer()),
                truncated: true,
            },
        )]);
        let verdict = CodeRecord::compare(3, &c, &answers);
        let states: Vec<SymbolState> = verdict.symbols.iter().map(|s| s.state).collect();
        assert_eq!(states, vec![SymbolState::Found, SymbolState::Unchecked]);
        assert_eq!(verdict.outcome(), crate::VerdictOutcome::Passed);
        assert_eq!(verdict.unchecked_files.len(), 1);
        let complete = BTreeMap::from([(
            "a.py".to_string(),
            FileAnswer::Symbols {
                found: CodeRecord::document_symbols(&pyright_answer()),
                truncated: false,
            },
        )]);
        let verdict = CodeRecord::compare(3, &c, &complete);
        assert_eq!(verdict.symbols[1].state, SymbolState::Missing);
        assert_eq!(verdict.outcome(), crate::VerdictOutcome::Failed);
        assert_eq!(
            verdict.lines(),
            "T3 promises logout in a.py; the code does not have it."
        );
    }

    #[test]
    fn a_dotted_name_matches_a_method_in_its_class() {
        let c = contract(
            &[("a.py", FileAction::Create)],
            &[
                ("s1", "a.py", "Auth.refresh"),
                ("s2", "a.py", "Other.refresh"),
                ("s3", "a.py", "refresh"),
                ("s4", "a.py", "Auth.token"),
            ],
        );
        let answers = BTreeMap::from([(
            "a.py".to_string(),
            FileAnswer::Symbols {
                found: CodeRecord::document_symbols(&pyright_answer()),
                truncated: false,
            },
        )]);
        let verdict = CodeRecord::compare(1, &c, &answers);
        let states: Vec<SymbolState> = verdict.symbols.iter().map(|s| s.state).collect();
        assert_eq!(
            states,
            vec![
                SymbolState::Found,
                SymbolState::Missing,
                SymbolState::Found,
                SymbolState::Missing
            ]
        );
        // `Auth` is named through `Auth.refresh`; `login` is named by no symbol.
        assert_eq!(
            verdict.unpromised,
            vec![Unpromised {
                path: "a.py".into(),
                name: "login".into()
            }]
        );
    }

    /// What rust-analyzer 1.97 answers for a struct with two `impl` blocks, a
    /// test module, and one free function.
    fn rust_analyzer_answer() -> Value {
        json!([
            { "name": "Store", "kind": 23, "children": [
                { "name": "notes", "kind": 8 },
                { "name": "next_id", "kind": 8 }
            ]},
            { "name": "impl Store", "kind": 19, "children": [
                { "name": "new", "kind": 12 },
                { "name": "add", "kind": 6 },
                { "name": "delete", "kind": 6 },
                { "name": "list_all", "kind": 6 }
            ]},
            { "name": "impl Default for Store", "kind": 19, "children": [
                { "name": "default", "kind": 12 }
            ]},
            { "name": "helper", "kind": 12 },
            { "name": "tests", "kind": 2, "children": [
                { "name": "add_gives_the_next_id", "kind": 12 }
            ]}
        ])
    }

    fn store_contract(path: &str, prefix: &str) -> TaskContract {
        let names: Vec<String> = ["new", "add", "delete", "list_all"]
            .iter()
            .map(|m| format!("{prefix}{m}"))
            .collect();
        let mut symbols = vec![("s0", path, "Store")];
        for (id, name) in ["s1", "s2", "s3", "s4"].iter().zip(&names) {
            symbols.push((id, path, name));
        }
        contract(&[(path, FileAction::Create)], &symbols)
    }

    #[test]
    fn a_rust_impl_block_gives_its_methods_their_type() {
        let c = store_contract("src/store.rs", "Store.");
        let answers = BTreeMap::from([(
            "src/store.rs".to_string(),
            FileAnswer::Symbols {
                found: CodeRecord::document_symbols(&rust_analyzer_answer()),
                truncated: false,
            },
        )]);
        let verdict = CodeRecord::compare(1, &c, &answers);
        let states: Vec<SymbolState> = verdict.symbols.iter().map(|s| s.state).collect();
        assert_eq!(states, vec![SymbolState::Found; 5], "{verdict:?}");
        // No impl block and no test module is unpromised; the free function is.
        assert_eq!(
            verdict.unpromised,
            vec![Unpromised {
                path: "src/store.rs".into(),
                name: "helper".into()
            }]
        );
    }

    #[test]
    fn an_impl_name_gives_its_type() {
        let cases = [
            ("impl Store", "Store"),
            ("impl<T> Store<T>", "Store"),
            ("impl Default for Store", "Store"),
            (
                "impl<T: Clone> From<Vec<T>> for crate::store::Store<T>",
                "Store",
            ),
            ("Auth", "Auth"),
        ];
        for (container, owner) in cases {
            assert_eq!(CodeRecord::owner(container), owner, "{container}");
        }
    }

    #[test]
    fn a_gopls_receiver_name_gives_its_type() {
        let flat = json!([
            { "name": "Store", "kind": 23, "location": {} },
            { "name": "(*Store).Add", "kind": 6, "location": {} },
            { "name": "(Store).Len", "kind": 6, "location": {} },
            { "name": "Reader", "kind": 11, "location": {} }
        ]);
        let tree = json!([
            { "name": "Store", "kind": 23, "children": [
                { "name": "items", "kind": 8 }
            ]},
            { "name": "(*Store).Add", "kind": 6 },
            { "name": "(Store).Len", "kind": 6 },
            { "name": "Reader", "kind": 11 }
        ]);
        let c = contract(
            &[("store.go", FileAction::Create)],
            &[
                ("s1", "store.go", "Store"),
                ("s2", "store.go", "Store.Add"),
                ("s3", "store.go", "Store.Len"),
            ],
        );
        for answer in [flat, tree] {
            let answers = BTreeMap::from([(
                "store.go".to_string(),
                FileAnswer::Symbols {
                    found: CodeRecord::document_symbols(&answer),
                    truncated: false,
                },
            )]);
            let verdict = CodeRecord::compare(1, &c, &answers);
            let states: Vec<SymbolState> = verdict.symbols.iter().map(|s| s.state).collect();
            assert_eq!(states, vec![SymbolState::Found; 3], "{verdict:?}");
            assert_eq!(
                verdict.unpromised,
                vec![Unpromised {
                    path: "store.go".into(),
                    name: "Reader".into()
                }]
            );
        }
    }
}

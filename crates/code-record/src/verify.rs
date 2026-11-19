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

/// The `documentSymbol` kinds that count: class, method, constructor, function.
const COUNTED: [u64; 4] = [5, 6, 9, 12];

/// One counted symbol of a `documentSymbol` answer.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Found {
    name: String,
    /// The counted symbol it is inside, if any.
    parent: Option<String>,
    top: bool,
}

/// What one file's question came back with.
#[derive(Debug)]
enum FileAnswer {
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
            let recorded = self
                .recorded
                .lock()
                .expect("recorded answers")
                .get(path)
                .cloned();
            match recorded {
                Some(crate::RecordedAnswer::Symbols(result)) => {
                    return FileAnswer::Symbols {
                        found: CodeRecord::document_symbols(&result),
                        truncated: false,
                    };
                }
                Some(crate::RecordedAnswer::NoServer) => {
                    return FileAnswer::Unchecked("no language server reads it".into());
                }
                Some(crate::RecordedAnswer::Absent) => return FileAnswer::Absent,
                None => {}
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
                } => {
                    return FileAnswer::Symbols {
                        found: CodeRecord::document_symbols(&result),
                        truncated,
                    };
                }
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

    /// The counted symbols of a `documentSymbol` result, at any depth: a
    /// `DocumentSymbol` tree (`children`) or a flat `SymbolInformation` list
    /// (`containerName`). A variable, parameter or field is never counted.
    fn document_symbols(result: &Value) -> Vec<Found> {
        fn walk(items: &Value, parent: Option<&str>, top: bool, out: &mut Vec<Found>) {
            for item in items.as_array().into_iter().flatten() {
                let name = item.get("name").and_then(Value::as_str);
                let kind = item.get("kind").and_then(Value::as_u64);
                let counted = kind.is_some_and(|k| COUNTED.contains(&k));
                let flat_parent = item.get("containerName").and_then(Value::as_str);
                let (parent, top) = match flat_parent {
                    Some(c) if !c.is_empty() => (Some(c), false),
                    _ => (parent, top),
                };
                if let (Some(name), true) = (name, counted) {
                    out.push(Found {
                        name: name.to_string(),
                        parent: parent.map(str::to_string),
                        top,
                    });
                }
                if let Some(children) = item.get("children") {
                    let next = if counted { name } else { parent };
                    walk(children, next, false, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(result, None, true, &mut out);
        out
    }

    /// Whether a contract name matches a counted symbol: exactly, at any
    /// depth, or `Class.method` as `method` inside `Class`.
    fn matches(name: &str, found: &Found) -> bool {
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
}

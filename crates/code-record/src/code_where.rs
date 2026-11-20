//! One job: answering where a topic is. The record answers first, from
//! CodeRecord's tables and Design's elements, with no language server; then
//! the code confirms it, within 30 asks and 20 s.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use codebase::{Asker, InventoryKind, InventoryQuery, LspAnswer, LspOperation, LspQuestion};
use serde_json::Value;
use tokio::time::Instant;

use core_design::{DesignElement, OwnedVia, ProjectLayout};
use core_plans::{ElementAction, WorkflowId};
use core_projects::ProjectId;

use crate::ranking::{Matches, Ranked, Ranking};
use crate::store::{Detail, SymbolRow};
use crate::verify::FileAnswer;
use crate::{
    CodeRecord, CodeRecordError, Confirmation, ElementEntry, FileEntry, FileTask, Live, Owner,
    SymbolEntry, SymbolState, UnrecordedFile, UnrecordedSymbol, UseLine, UsesLive, WhereAnswer,
    WhereEntry,
};

/// The longest topic a question takes, in characters.
const TOPIC_MAX: usize = 200;

impl CodeRecord {
    /// Where `topic` is in `project`: the record's entries and elements, ranked,
    /// then what the code holds of them now.
    pub async fn code_where(
        &self,
        project: &ProjectId,
        topic: &str,
    ) -> Result<WhereAnswer, CodeRecordError> {
        let topic = topic.trim();
        if topic.is_empty() || topic.chars().count() > TOPIC_MAX {
            return Err(CodeRecordError::InvalidTopic(
                "a topic needs 1 to 200 characters".into(),
            ));
        }
        let layout = self.design.layout(project).await?;
        let elements = self.design.elements(project).await?;
        let path = topic.trim_end_matches('/');
        let folders: Vec<String> = Ranking::parts(topic, &layout)
            .into_iter()
            .map(|part| part.folder.clone())
            .collect();
        let pinned = self
            .store
            .element_pins(project)
            .await?
            .into_iter()
            .map(|(id, pins)| {
                let files = pins
                    .into_iter()
                    .filter(|pin| pin.action != ElementAction::Delete)
                    .map(|pin| pin.file)
                    .collect();
                (id, files)
            })
            .collect();
        let matches = Matches {
            symbol_paths: self.store.symbols_named(project, topic).await?,
            path_paths: self.store.files_by_path(project, path).await?,
            part_paths: self.store.files_under(project, &folders).await?,
            word_paths: self
                .store
                .rows_with_words(project, &Ranking::words(topic))
                .await?,
            pinned,
        };
        let (ranked, entries_more) = Ranking::entries(topic, &layout, &elements, &matches);
        let paths: Vec<String> = ranked
            .iter()
            .filter_map(|entry| match entry {
                Ranked::File { path, .. } => Some(path.clone()),
                Ranked::Element { .. } => None,
            })
            .collect();
        let detail = self.store.files_detail(project, &paths).await?;
        let matched: Vec<DesignElement> = ranked
            .iter()
            .filter_map(|entry| match entry {
                Ranked::Element { index, .. } => Some(elements[*index].clone()),
                Ranked::File { .. } => None,
            })
            .collect();
        let mut states = self
            .element_states(project, &matched, &[])
            .await?
            .into_iter();
        let mut matched = matched.into_iter();
        let mut entries = Vec::with_capacity(ranked.len());
        for entry in ranked {
            entries.push(match entry {
                Ranked::File { rank, path } => {
                    WhereEntry::File(file_entry(rank, path, &detail, &layout)?)
                }
                Ranked::Element { rank, .. } => WhereEntry::Element(ElementEntry {
                    rank,
                    element: matched.next().expect("one matched element per entry"),
                    state: states.next().expect("one state per matched element"),
                }),
            });
        }
        let mut answer = WhereAnswer {
            topic: topic.to_string(),
            entries,
            entries_more,
            not_in_record: Vec::new(),
            not_in_record_more: 0,
            asked: 0,
            budget: None,
        };
        self.confirm(project, &layout, &mut answer).await?;
        Ok(answer)
    }
}

/// A recorded file with its tasks, newest first, and the newest row of each
/// of its symbols, by name.
fn file_entry(
    rank: crate::Rank,
    path: String,
    detail: &Detail,
    layout: &ProjectLayout,
) -> Result<FileEntry, CodeRecordError> {
    let tasks = detail
        .files
        .iter()
        .filter(|row| row.path == path)
        .map(|row| FileTask {
            workflow_id: WorkflowId::from_recorded(row.workflow_id.clone()),
            version: row.version,
            task: u32::try_from(row.task).unwrap_or_default(),
            action: row.action.clone(),
            purpose: row.purpose.clone(),
            commit: row.commit.clone(),
        })
        .collect();
    let mut seen = BTreeSet::new();
    let mut symbols = Vec::new();
    for row in detail.symbols.iter().filter(|row| row.path == path) {
        if seen.insert(row.name.as_str()) {
            symbols.push(symbol_entry(row)?);
        }
    }
    symbols.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(FileEntry {
        rank,
        owner: CodeRecord::path_owner(layout, &path),
        path,
        tasks,
        symbols,
        live: Live::NotAsked,
        confirmed: None,
    })
}

fn symbol_entry(row: &SymbolRow) -> Result<SymbolEntry, CodeRecordError> {
    let contract: serde_json::Value = serde_json::from_str(&row.contract_json)?;
    let behavior = contract
        .get("behavior")
        .and_then(|b| b.as_array())
        .map(|lines| {
            lines
                .iter()
                .filter_map(|line| line.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let at_done: SymbolState =
        serde_json::from_value(serde_json::Value::String(row.state.clone()))?;
    Ok(SymbolEntry {
        symbol_id: row.symbol_id.clone(),
        name: row.name.clone(),
        kind: row.kind.clone(),
        workflow_id: WorkflowId::from_recorded(row.workflow_id.clone()),
        version: row.version,
        task: u32::try_from(row.task).unwrap_or_default(),
        behavior,
        at_done,
        live: Live::NotAsked,
        line: None,
        uses_live: UsesLive::NotAsked,
        uses: Vec::new(),
        uses_more: 0,
        uses_total: None,
    })
}

impl CodeRecord {
    /// The part whose folder holds `path`, and the first layer whose place does.
    fn path_owner(layout: &ProjectLayout, path: &str) -> Option<Owner> {
        let owners = layout.owners(path);
        let part = owners
            .iter()
            .find(|o| o.via == OwnedVia::Folder)?
            .part
            .clone();
        let layer = owners.iter().find_map(|o| match &o.via {
            OwnedVia::Layer(id) => Some(id.clone()),
            OwnedVia::Folder => None,
        });
        let part_title = layout
            .parts
            .iter()
            .find(|p| p.id == part)
            .map(|p| p.title.clone())
            .unwrap_or_default();
        Some(Owner {
            part_id: part,
            part_title,
            layer,
        })
    }
}

/// The most language-server asks one question spends.
const MAX_ASKS: u32 = 30;
/// The longest the live half of one question takes.
const LIVE_FOR: Duration = Duration::from_secs(20);
/// The most uses listed per symbol.
const USES_MAX: usize = 10;
/// The most files not in the record listed.
const UNRECORDED_MAX: usize = 10;
/// The page of the file list read for a path topic.
const LIST_PAGE: u16 = 200;

const BUDGET: &str = "the question's budget ran out (30 asks, 20 s)";
const CUT: &str = "the server's answer was cut at 100 items or 32 KiB; there may be more";
const LATE: &str = "the question's 20 s ran out during the lookup";

/// What one live ask came back with.
enum Asked {
    Answered {
        result: Value,
        truncated: bool,
    },
    NoServer(String),
    Failed(String),
    /// The deadline passed during the ask.
    Late,
}

/// The asks and time one question has left.
struct Budget {
    asked: u32,
    deadline: Instant,
    /// Something was not asked because the budget was spent.
    ran_out: bool,
}

impl Budget {
    fn left(&self) -> bool {
        self.asked < MAX_ASKS && Instant::now() < self.deadline
    }
}

impl CodeRecord {
    /// The code's half: each file and symbol confirmed now, within the budget,
    /// and the files no contract names.
    async fn confirm(
        &self,
        project: &ProjectId,
        layout: &ProjectLayout,
        answer: &mut WhereAnswer,
    ) -> Result<(), CodeRecordError> {
        let mut budget = Budget {
            asked: 0,
            deadline: Instant::now() + LIVE_FOR,
            ran_out: false,
        };
        let paths: Vec<String> = answer
            .entries
            .iter()
            .filter_map(|entry| match entry {
                WhereEntry::File(file) => Some(file.path.clone()),
                WhereEntry::Element(_) => None,
            })
            .collect();
        if !paths.is_empty() {
            match self
                .code
                .missing_inputs(Asker::Person(project), &paths)
                .await
            {
                Ok(absent) => {
                    for file in files_mut(&mut answer.entries) {
                        if absent.contains(&file.path) {
                            file.live = Live::Gap {
                                why: format!("{} is absent now", file.path),
                            };
                            for symbol in &mut file.symbols {
                                symbol.live = Live::Gap {
                                    why: "its file is absent now".into(),
                                };
                            }
                        }
                    }
                }
                Err(error) => {
                    let why = format!("the folder could not be read: {error}");
                    for file in files_mut(&mut answer.entries) {
                        unchecked(file, &why);
                    }
                    return Ok(());
                }
            }
        }
        let topic = answer.topic.clone();
        if topic.contains('/') {
            self.unrecorded_in_list(project, layout, &topic, answer)
                .await?;
        } else if !paths.contains(&topic) && budget.left() {
            self.unrecorded_symbols(project, layout, &topic, answer, &mut budget)
                .await?;
        }
        for file in files_mut(&mut answer.entries) {
            if file.live != Live::NotAsked {
                continue;
            }
            if file.symbols.is_empty() {
                file.live = Live::Confirmed;
                file.confirmed = Some(Confirmation::FileOnly);
                continue;
            }
            self.confirm_file(project, layout, file, &mut budget).await;
        }
        answer.asked = budget.asked;
        if budget.ran_out {
            answer.budget = Some(BUDGET.into());
        }
        Ok(())
    }

    /// One file's symbols by `documentSymbol`, then each confirmed symbol's uses.
    async fn confirm_file(
        &self,
        project: &ProjectId,
        layout: &ProjectLayout,
        file: &mut FileEntry,
        budget: &mut Budget,
    ) {
        if !budget.left() {
            budget.ran_out = true;
            unchecked(file, BUDGET);
            return;
        }
        let question = LspQuestion {
            operation: LspOperation::DocumentSymbol,
            file: Some(file.path.clone()),
            line: None,
            character: None,
            query: None,
            project: None,
        };
        let mut columns = BTreeMap::new();
        match self.ask_now(project, question, &file.path, budget).await {
            Asked::Answered { result, truncated } => {
                let FileAnswer::Symbols { found, .. } = CodeRecord::file_answer(&result, truncated)
                else {
                    return;
                };
                for symbol in &mut file.symbols {
                    match found.iter().find(|f| CodeRecord::matches(&symbol.name, f)) {
                        Some(f) => {
                            symbol.live = Live::Confirmed;
                            symbol.line = f.line;
                            columns.insert(symbol.symbol_id.clone(), f.character.unwrap_or(1));
                        }
                        None if truncated => {
                            symbol.live = Live::Unchecked {
                                why: "the server's answer was cut short".into(),
                            }
                        }
                        None => {
                            symbol.live = Live::Gap {
                                why: format!("{} is not in {} now", symbol.name, file.path),
                            }
                        }
                    }
                }
                file.live = Live::Confirmed;
                file.confirmed = Some(Confirmation::FileAndSymbol);
            }
            Asked::NoServer(why) | Asked::Failed(why) => unchecked(file, &why),
            Asked::Late => {
                budget.ran_out = true;
                unchecked(file, BUDGET);
            }
        }
        for symbol in &mut file.symbols {
            let (Live::Confirmed, Some(line)) = (&symbol.live, symbol.line) else {
                continue;
            };
            if !budget.left() {
                budget.ran_out = true;
                symbol.uses_live = UsesLive::Skipped { why: BUDGET.into() };
                continue;
            }
            let question = LspQuestion {
                operation: LspOperation::FindReferences,
                file: Some(file.path.clone()),
                line: Some(line),
                character: columns.get(&symbol.symbol_id).copied(),
                query: None,
                project: None,
            };
            let key = format!("{}#references@{line}", file.path);
            match self.ask_now(project, question, &key, budget).await {
                Asked::Answered { result, truncated } => {
                    let mut uses: Vec<(String, u32)> = result
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|item| {
                            let path = item.get("path")?.as_str()?.to_string();
                            let at = u32::try_from(item.get("line")?.as_u64()?).ok()?;
                            Some((path, at))
                        })
                        .filter(|(path, at)| !(path == &file.path && *at == line))
                        .collect();
                    uses.sort();
                    let total = u32::try_from(uses.len()).unwrap_or(u32::MAX);
                    symbol.uses_more = total.saturating_sub(USES_MAX as u32);
                    symbol.uses = uses
                        .into_iter()
                        .take(USES_MAX)
                        .map(|(path, line)| UseLine {
                            owner: CodeRecord::path_owner(layout, &path),
                            path,
                            line,
                        })
                        .collect();
                    if truncated {
                        symbol.uses_live = UsesLive::Truncated { why: CUT.into() };
                    } else {
                        symbol.uses_live = UsesLive::Answered;
                        symbol.uses_total = Some(total);
                    }
                }
                Asked::NoServer(why) | Asked::Failed(why) => {
                    symbol.uses_live = UsesLive::Failed { why };
                }
                Asked::Late => {
                    budget.ran_out = true;
                    symbol.uses_live = UsesLive::Failed { why: LATE.into() };
                }
            }
        }
    }

    /// Files under a path topic that no done task's contract names, from the file list.
    async fn unrecorded_in_list(
        &self,
        project: &ProjectId,
        layout: &ProjectLayout,
        topic: &str,
        answer: &mut WhereAnswer,
    ) -> Result<(), CodeRecordError> {
        let query = InventoryQuery {
            prefix: topic.trim_end_matches('/').to_string(),
            after: None,
            limit: LIST_PAGE,
        };
        let page = match self.code.inventory(Asker::Person(project), query).await {
            Ok(page) => page,
            Err(error) => {
                answer.budget = Some(format!("the file list could not be read: {error}"));
                return Ok(());
            }
        };
        let listed: Vec<String> = page
            .entries
            .into_iter()
            .filter(|entry| entry.kind == InventoryKind::File)
            .map(|entry| entry.path)
            .collect();
        let recorded = self.recorded_paths(project, &listed).await?;
        let mut files: Vec<UnrecordedFile> = listed
            .into_iter()
            .filter(|path| !recorded.contains(path))
            .map(|path| UnrecordedFile {
                owner: CodeRecord::path_owner(layout, &path),
                path,
                symbols: Vec::new(),
            })
            .collect();
        answer.not_in_record_more = more_than(files.len(), UNRECORDED_MAX);
        files.truncate(UNRECORDED_MAX);
        answer.not_in_record = files;
        Ok(())
    }

    /// Files holding a symbol named like the topic that no done task's
    /// contract names, from one `workspaceSymbol` ask.
    async fn unrecorded_symbols(
        &self,
        project: &ProjectId,
        layout: &ProjectLayout,
        topic: &str,
        answer: &mut WhereAnswer,
        budget: &mut Budget,
    ) -> Result<(), CodeRecordError> {
        let question = LspQuestion {
            operation: LspOperation::WorkspaceSymbol,
            file: None,
            line: None,
            character: None,
            query: Some(topic.to_string()),
            project: None,
        };
        let key = format!("#workspace:{topic}");
        let result = match self.ask_now(project, question, &key, budget).await {
            Asked::Answered { result, .. } => result,
            Asked::Late => {
                budget.ran_out = true;
                return Ok(());
            }
            Asked::NoServer(_) | Asked::Failed(_) => return Ok(()),
        };
        let mut by_path: BTreeMap<String, Vec<UnrecordedSymbol>> = BTreeMap::new();
        for item in result.as_array().into_iter().flatten() {
            let (Some(name), Some(path), Some(line)) = (
                item.get("name").and_then(Value::as_str),
                item.get("path").and_then(Value::as_str),
                item.get("line")
                    .and_then(Value::as_u64)
                    .and_then(|n| u32::try_from(n).ok()),
            ) else {
                continue;
            };
            by_path
                .entry(path.to_string())
                .or_default()
                .push(UnrecordedSymbol {
                    name: name.to_string(),
                    line,
                });
        }
        let paths: Vec<String> = by_path.keys().cloned().collect();
        let recorded = self.recorded_paths(project, &paths).await?;
        let mut files: Vec<UnrecordedFile> = by_path
            .into_iter()
            .filter(|(path, _)| !recorded.contains(path))
            .map(|(path, mut symbols)| {
                symbols.sort_by(|a, b| (a.line, &a.name).cmp(&(b.line, &b.name)));
                UnrecordedFile {
                    owner: CodeRecord::path_owner(layout, &path),
                    path,
                    symbols,
                }
            })
            .collect();
        answer.not_in_record_more = more_than(files.len(), UNRECORDED_MAX);
        files.truncate(UNRECORDED_MAX);
        answer.not_in_record = files;
        Ok(())
    }

    /// Which of `paths` a done task's contract names.
    async fn recorded_paths(
        &self,
        project: &ProjectId,
        paths: &[String],
    ) -> Result<BTreeSet<String>, CodeRecordError> {
        Ok(self
            .store
            .files_detail(project, paths)
            .await?
            .files
            .into_iter()
            .map(|row| row.path)
            .collect())
    }

    /// One language-server ask with no wait for a starting server, counted
    /// against the budget and bounded by its deadline. A test's recorded
    /// answer under `key` stands in for the server.
    async fn ask_now(
        &self,
        project: &ProjectId,
        question: LspQuestion,
        key: &str,
        budget: &mut Budget,
    ) -> Asked {
        budget.asked += 1;
        #[cfg(feature = "test-support")]
        if let Some(recorded) = self.recorded(key) {
            use crate::RecordedAnswer;
            return match recorded {
                RecordedAnswer::Symbols(result) | RecordedAnswer::Workspace(result) => {
                    Asked::Answered {
                        result,
                        truncated: false,
                    }
                }
                RecordedAnswer::References { result, truncated } => {
                    Asked::Answered { result, truncated }
                }
                RecordedAnswer::NoServer => {
                    let path = question.file.as_deref().unwrap_or_default();
                    let extension = path
                        .rsplit('/')
                        .next()
                        .and_then(|name| name.rfind('.').filter(|&i| i > 0).map(|i| &name[i..]))
                        .unwrap_or_default();
                    Asked::NoServer(format!("no language server reads {extension}"))
                }
                RecordedAnswer::Absent => Asked::Failed(crate::model::ABSENT.into()),
                RecordedAnswer::Failed(why) => Asked::Failed(why),
            };
        }
        #[cfg(not(feature = "test-support"))]
        let _ = key;
        let ask = self.code.lsp(Asker::Person(project), question);
        match tokio::time::timeout_at(budget.deadline, ask).await {
            Err(_) => Asked::Late,
            Ok(Err(error)) => Asked::Failed(error.to_string()),
            Ok(Ok(LspAnswer::Answered {
                result, truncated, ..
            })) => Asked::Answered { result, truncated },
            Ok(Ok(LspAnswer::NoServer {
                extension, install, ..
            })) => {
                let mut why = format!("no language server reads {extension}");
                if install.is_some() {
                    why.push_str(" (not installed)");
                }
                Asked::NoServer(why)
            }
        }
    }
}

/// Every file entry, in rank order.
fn files_mut(entries: &mut [WhereEntry]) -> impl Iterator<Item = &mut FileEntry> {
    entries.iter_mut().filter_map(|entry| match entry {
        WhereEntry::File(file) => Some(file),
        WhereEntry::Element(_) => None,
    })
}

/// The file and every symbol of it not checked now, for `why`.
fn unchecked(file: &mut FileEntry, why: &str) {
    file.live = Live::Unchecked { why: why.into() };
    for symbol in &mut file.symbols {
        symbol.live = Live::Unchecked { why: why.into() };
    }
}

fn more_than(count: usize, max: usize) -> u32 {
    u32::try_from(count.saturating_sub(max)).unwrap_or(u32::MAX)
}

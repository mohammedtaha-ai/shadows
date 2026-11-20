//! One job: answering where a topic is. The record answers first, from
//! CodeRecord's tables and Design's elements, with no language server.

use std::collections::BTreeSet;

use core_design::{DesignElement, OwnedVia, ProjectLayout};
use core_plans::{ElementAction, WorkflowId};
use core_projects::ProjectId;

use crate::ranking::{Matches, Ranked, Ranking};
use crate::store::{Detail, SymbolRow};
use crate::{
    CodeRecord, CodeRecordError, ElementEntry, FileEntry, FileTask, Live, Owner, SymbolEntry,
    SymbolState, UsesLive, WhereAnswer, WhereEntry,
};

/// The longest topic a question takes, in characters.
const TOPIC_MAX: usize = 200;

impl CodeRecord {
    /// Where `topic` is in `project`: the record's entries and elements, ranked.
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
        Ok(WhereAnswer {
            topic: topic.to_string(),
            entries,
            entries_more,
            not_in_record: Vec::new(),
            not_in_record_more: 0,
            asked: 0,
            budget: None,
        })
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

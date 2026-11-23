//! One job: the code record — whether the code holds what a task's contract
//! promised, and the verdicts that say so.
//!
//! Runs owns its check rows and calls `verify`; this service never calls Runs.
//! It asks Design for elements and the layout when answering `code_where`
//! and when writing the record's `digest`.

mod code_where;
mod digest;
mod elements;
mod error;
mod model;
mod ranking;
mod store;
mod verify;

use std::sync::Arc;

use codebase::Codebase;
use core_db::Storage;
use core_design::Design;
use store::CodeRecordStore;

pub use error::CodeRecordError;
pub use model::{
    BuildState, Confirmation, DigestFile, DigestGap, DigestPart, DigestSymbol, DoneRecord,
    ElementEntry, ElementState, ElementTask, FileEntry, FileTask, Live, Owner, PlannedPin,
    PromisedSymbol, Rank, RecordDigest, SymbolEntry, SymbolState, UncheckedFile, Unpromised,
    UnrecordedFile, UnrecordedSymbol, UseLine, UsesLive, Verdict, VerdictOutcome, WhereAnswer,
    WhereEntry,
};

/// What a test records as one file's answer, in place of a language server's.
#[cfg(feature = "test-support")]
#[derive(Debug, Clone)]
pub enum RecordedAnswer {
    /// A `documentSymbol` result, not truncated.
    Symbols(serde_json::Value),
    /// No language server reads the file.
    NoServer,
    /// The file does not exist.
    Absent,
    /// A `findReferences` result, under `"{path}#references@{line}"`; `truncated`
    /// stands for Codebase's bound cutting it.
    References {
        result: serde_json::Value,
        truncated: bool,
    },
    /// Under any key, the ask fails with this text, as a language server error would.
    Failed(String),
    /// A `workspaceSymbol` result, under `"#workspace:{query}"`.
    Workspace(serde_json::Value),
}

/// Cheap to clone: its private store, the `Codebase` it asks and the `Design` it reads.
#[derive(Clone)]
pub struct CodeRecord {
    store: CodeRecordStore,
    code: Codebase,
    design: Design,
    /// Answers recorded by tests, by path; asked before any language server.
    #[cfg(feature = "test-support")]
    recorded: Arc<std::sync::Mutex<std::collections::HashMap<String, RecordedAnswer>>>,
}

impl CodeRecord {
    pub fn new(storage: Arc<Storage>, code: Codebase, design: Design) -> Self {
        Self {
            store: CodeRecordStore::new(storage),
            code,
            design,
            #[cfg(feature = "test-support")]
            recorded: Arc::default(),
        }
    }

    /// The verdict stored under `key`, if one was.
    pub async fn verdict(&self, key: &str) -> Result<Option<Verdict>, CodeRecordError> {
        self.store.verdict(key).await
    }

    /// Answers `key` with `answer` from now on, in place of asking a server: a
    /// path for `documentSymbol`, `"{path}#references@{line}"` or `"#workspace:{query}"`.
    #[cfg(feature = "test-support")]
    pub fn record_answer_for_tests(&self, key: &str, answer: RecordedAnswer) {
        self.recorded
            .lock()
            .expect("recorded answers")
            .insert(key.to_string(), answer);
    }
}

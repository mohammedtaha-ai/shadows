//! One job: the code record — whether the code holds what a task's contract
//! promised, and the verdicts that say so.
//!
//! Runs owns its check rows and calls `verify`; this service never calls Runs.

mod elements;
mod error;
mod model;
mod store;
mod verify;

use std::sync::Arc;

use codebase::Codebase;
use core_db::Storage;
use store::CodeRecordStore;

pub use error::CodeRecordError;
pub use model::{
    BuildState, Confirmation, DoneRecord, ElementState, ElementTask, PlannedPin, PromisedSymbol,
    SymbolState, UncheckedFile, Unpromised, Verdict, VerdictOutcome,
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
}

/// Cheap to clone: its private store and the `Codebase` it asks.
#[derive(Clone)]
pub struct CodeRecord {
    store: CodeRecordStore,
    code: Codebase,
    /// Answers recorded by tests, by path; asked before any language server.
    #[cfg(feature = "test-support")]
    recorded: Arc<std::sync::Mutex<std::collections::HashMap<String, RecordedAnswer>>>,
}

impl CodeRecord {
    pub fn new(storage: Arc<Storage>, code: Codebase) -> Self {
        Self {
            store: CodeRecordStore::new(storage),
            code,
            #[cfg(feature = "test-support")]
            recorded: Arc::default(),
        }
    }

    /// The verdict stored under `key`, if one was.
    pub async fn verdict(&self, key: &str) -> Result<Option<Verdict>, CodeRecordError> {
        self.store.verdict(key).await
    }

    /// Answers `path` with `answer` from now on, in place of asking a server.
    #[cfg(feature = "test-support")]
    pub fn record_answer_for_tests(&self, path: &str, answer: RecordedAnswer) {
        self.recorded
            .lock()
            .expect("recorded answers")
            .insert(path.to_string(), answer);
    }
}

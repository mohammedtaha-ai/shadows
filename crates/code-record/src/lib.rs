//! One job: the code record — whether the code holds what a task's contract
//! promised, and the verdicts that say so.
//!
//! Runs owns its check rows and calls `verify`; this service never calls Runs.

mod error;
mod model;
mod store;
mod verify;

use std::sync::Arc;

use codebase::Codebase;
use core_db::Storage;
use store::CodeRecordStore;

pub use error::CodeRecordError;
pub use model::{PromisedSymbol, SymbolState, UncheckedFile, Unpromised, Verdict, VerdictOutcome};

/// Cheap to clone: its private store and the `Codebase` it asks.
#[derive(Clone)]
pub struct CodeRecord {
    store: CodeRecordStore,
    code: Codebase,
}

impl CodeRecord {
    pub fn new(storage: Arc<Storage>, code: Codebase) -> Self {
        Self {
            store: CodeRecordStore::new(storage),
            code,
        }
    }

    /// The verdict stored under `key`, if one was.
    pub async fn verdict(&self, key: &str) -> Result<Option<Verdict>, CodeRecordError> {
        self.store.verdict(key).await
    }
}

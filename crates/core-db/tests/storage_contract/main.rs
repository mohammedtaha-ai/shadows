//! The shared storage contract, with one module per persistence concern.

mod journal;
mod schema;
mod txn;

#[path = "../fixtures/journal.rs"]
mod fixtures;

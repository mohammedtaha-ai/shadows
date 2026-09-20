//! Top-level migration entry point for the SQLx spike.
//!
//! Currently a thin re-export — migrations live in storage::sqlite::migrations.

pub use crate::storage::sqlite::migrations::run_migrations as migrate_sqlite;

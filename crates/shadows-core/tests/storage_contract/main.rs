//! The storage layer every service shares (spec §6.23, §2.4): one test
//! binary, one module per job.

mod journal;
mod schema;
mod txn;

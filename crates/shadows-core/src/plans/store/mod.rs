//! One job: plans' SQLite queries (spec §14.4). Each file is `impl Storage`
//! over the shared pool, which stays in `db`: `edit.rs` changes a
//! version, `draft.rs` starts one, `read.rs` reads them, `task.rs` holds a
//! version's task and link rows, `view.rs` shows one in its conversation.
//!
//! What another service's store may call inside one of its writes is
//! `pub(crate)` and declared in `plans/contract.yaml` under
//! `shared_in_transaction`; nothing else here is.

mod draft;
mod edit;
mod read;
mod task;
mod view;

pub(crate) use task::task_of;

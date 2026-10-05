//! One job: turns' SQLite queries (spec §14.4). Each file is `impl Storage`
//! over the shared pool, which stays in `db`: `turn.rs` starts a turn as
//! one command, `operation.rs` writes an operation's transitions,
//! `operation_read.rs` reads operations back, `transition.rs` records a
//! transition's event and log line, `queue.rs` keeps the waiting messages,
//! `steer.rs` records a steered one.
//!
//! What another write may call inside its own transaction is `pub(crate)` and
//! declared in `turns/contract.yaml` under `shared_in_transaction`; nothing
//! else here is.

mod operation;
mod operation_read;
mod queue;
mod steer;
mod transition;
mod turn;

pub use queue::{NewQueued, QueueAnswer};
pub(crate) use transition::{existed, read_before, record};
pub(crate) use turn::has_open_operation;
pub use turn::{Dequeue, NewTurn, StartedTurn};

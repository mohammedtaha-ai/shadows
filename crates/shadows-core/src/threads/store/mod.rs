//! One job: threads' SQLite queries — the thread row (`thread`), its entries
//! (`entry`), and a fork of it (`fork`).

mod entry;
mod fork;
mod thread;

// What another write calls inside its own transaction (spec §14.6).
pub(crate) use entry::append_entry_in;
pub(crate) use thread::insert_thread;

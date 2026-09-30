//! One job: threads' SQLite queries — the thread row (`thread`), its entries
//! (`entry`), a fork of it (`fork`), and its title after creation (`title`).

mod entry;
mod fork;
mod thread;
mod title;

// What another write calls inside its own transaction (spec §14.6).
pub(crate) use entry::append_entry_in;
pub(crate) use thread::insert_thread;
pub(crate) use title::title_from_first_message_in;

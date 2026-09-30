//! One job: turning one file's text into its tags (spec §15.3). It knows
//! nothing of SQLite, projects or the daemon.

mod extract;
mod languages;

pub use extract::{Role, Tag, extract};
pub use languages::{Language, language_for};

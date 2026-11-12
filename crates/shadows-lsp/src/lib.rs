//! One language server's process and its requests. It knows no project: the caller
//! names the executable, the root and each file's text.

mod client;

use std::path::PathBuf;

pub use client::Server;

/// How to start one language server: its command and the folder it serves.
#[derive(Debug, Clone)]
pub struct ServerSpec {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub root: PathBuf,
    pub env: Vec<(String, String)>,
}

/// What happened to a file on disk, told to the server so it reads it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChange {
    Created,
    Changed,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LspError {
    #[error("the language server did not start: {0}")]
    Spawn(String),
    #[error("the language server refused to initialize: {0}")]
    Initialize(String),
    #[error("the language server did not answer in time")]
    Timeout,
    #[error(
        "the language server exited{}",
        .0.as_ref().map(|l| format!(": {l}")).unwrap_or_default()
    )]
    Exited(Option<String>),
    #[error("the language server answered an error {code}: {message}")]
    Server { code: i64, message: String },
}

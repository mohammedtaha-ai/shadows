//! One job: the catalog of language servers Shadows knows, read from `catalog.json`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::Codebase;

/// One catalog entry, as `catalog.json` writes it.
#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct CatalogEntry {
    pub(crate) id: String,
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    /// Extension, as written (`.go`, `.C`), to the language id the server is told.
    pub(crate) extensions: BTreeMap<String, String>,
    pub(crate) install: Install,
}

/// The command that installs a server, per system; `None` when none is known.
#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct Install {
    windows: Option<String>,
    macos: Option<String>,
    linux: Option<String>,
}

impl Install {
    /// This system's line.
    pub(crate) fn here(&self) -> Option<String> {
        if cfg!(windows) {
            self.windows.clone()
        } else if cfg!(target_os = "macos") {
            self.macos.clone()
        } else {
            self.linux.clone()
        }
    }
}

impl Codebase {
    /// The catalog, parsed once. The file is part of the crate, so a parse failure is a
    /// build defect the catalog test catches.
    pub(crate) fn catalog() -> &'static [CatalogEntry] {
        static CATALOG: OnceLock<Vec<CatalogEntry>> = OnceLock::new();
        CATALOG.get_or_init(|| {
            serde_json::from_str(include_str!("catalog.json")).expect("catalog.json parses")
        })
    }
}

//! One job: the table of languages Shadows indexes (spec §15.3). Adding a
//! language is one entry here, its grammar crate, and its extra query file.

use std::path::Path;
use std::sync::LazyLock;

use tree_sitter_tags::TagsConfiguration;

/// Patterns the grammars' own queries miss
/// (docs/evidence/milestone3/PROBE.md at 92e6dae).
/// Always after the grammar's query: the earlier pattern wins a name.
const RUST_EXTRA: &str = include_str!("../queries/rust.scm");
const TS_EXTRA: &str = include_str!("../queries/typescript.scm");
const JS_EXTRA: &str = include_str!("../queries/javascript.scm");

pub struct Language {
    pub name: &'static str,
    /// Without the dot, lower case.
    pub extensions: &'static [&'static str],
    pub(crate) config: TagsConfiguration,
}

fn config(language: tree_sitter::Language, queries: &[&str]) -> TagsConfiguration {
    TagsConfiguration::new(language, &queries.join("\n"), "")
        .expect("a compiled-in tags query is valid")
}

static LANGUAGES: LazyLock<Vec<Language>> = LazyLock::new(|| {
    let (ts, js) = (
        tree_sitter_typescript::TAGS_QUERY,
        tree_sitter_javascript::TAGS_QUERY,
    );
    vec![
        Language {
            name: "rust",
            extensions: &["rs"],
            config: config(
                tree_sitter_rust::LANGUAGE.into(),
                &[tree_sitter_rust::TAGS_QUERY, RUST_EXTRA],
            ),
        },
        Language {
            name: "typescript",
            extensions: &["ts", "mts", "cts"],
            config: config(
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                &[ts, js, TS_EXTRA, JS_EXTRA],
            ),
        },
        Language {
            name: "tsx",
            extensions: &["tsx"],
            config: config(
                tree_sitter_typescript::LANGUAGE_TSX.into(),
                &[ts, js, TS_EXTRA, JS_EXTRA],
            ),
        },
        Language {
            name: "javascript",
            extensions: &["js", "mjs", "cjs", "jsx"],
            config: config(tree_sitter_javascript::LANGUAGE.into(), &[js, JS_EXTRA]),
        },
    ]
});

/// The language a path's extension names, or None.
pub fn language_for(path: &Path) -> Option<&'static Language> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    LANGUAGES
        .iter()
        .find(|l| l.extensions.contains(&ext.as_str()))
}

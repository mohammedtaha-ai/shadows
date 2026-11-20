//! One job: ranking a question's matches. Four fixed tiers, no scores.

use std::collections::BTreeMap;

use core_design::{DesignElement, ElementContent, LayoutPart, ProjectLayout};

use crate::Rank;

/// The most entries one answer lists; the rest are counted.
pub(crate) const ENTRIES_MAX: usize = 20;

/// What the record's reads matched, by path.
#[derive(Default)]
pub(crate) struct Matches {
    /// Files holding a recorded symbol named as the topic.
    pub symbol_paths: Vec<String>,
    /// Files at or under the topic as a path.
    pub path_paths: Vec<String>,
    /// Files under the folder of a part titled as the topic.
    pub part_paths: Vec<String>,
    /// Files whose purpose, or a symbol's contract, holds every word.
    pub word_paths: Vec<String>,
    /// Each element's files, from its done pins.
    pub pinned: BTreeMap<String, Vec<String>>,
}

/// One ranked entry: a recorded file, or an element by its index in the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Ranked {
    File { rank: Rank, path: String },
    Element { rank: Rank, index: usize },
}

pub(crate) struct Ranking;

impl Ranking {
    /// The topic's words: split on anything not a letter or digit, lowercased,
    /// 3 characters or more.
    pub(crate) fn words(topic: &str) -> Vec<String> {
        topic
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() >= 3)
            .map(str::to_lowercase)
            .collect()
    }

    /// The parts titled as the topic, ignoring case.
    pub(crate) fn parts<'a>(topic: &str, layout: &'a ProjectLayout) -> Vec<&'a LayoutPart> {
        layout
            .parts
            .iter()
            .filter(|part| part.title.to_lowercase() == topic.to_lowercase())
            .collect()
    }

    /// Every match at its best tier, once, ordered by tier, then files by
    /// path before elements by number; at most `ENTRIES_MAX`, and the count of the rest.
    pub(crate) fn entries(
        topic: &str,
        layout: &ProjectLayout,
        elements: &[DesignElement],
        matches: &Matches,
    ) -> (Vec<Ranked>, u32) {
        let lowered = topic.to_lowercase();
        let words = Self::words(topic);
        let parts: Vec<_> = Self::parts(topic, layout)
            .into_iter()
            .map(|p| p.id.clone())
            .collect();
        let mut files: BTreeMap<&str, Rank> = BTreeMap::new();
        for (paths, rank) in [
            (&matches.symbol_paths, Rank::Exact),
            (&matches.path_paths, Rank::Path),
            (&matches.part_paths, Rank::Part),
            (&matches.word_paths, Rank::Words),
        ] {
            for path in paths {
                let best = files.entry(path.as_str()).or_insert(rank);
                *best = (*best).min(rank);
            }
        }
        let mut ranked: Vec<(Rank, u8, String, u64, Ranked)> = files
            .into_iter()
            .map(|(path, rank)| {
                (
                    rank,
                    0,
                    path.to_string(),
                    0,
                    Ranked::File {
                        rank,
                        path: path.to_string(),
                    },
                )
            })
            .collect();
        for (index, element) in elements.iter().enumerate() {
            let (name, purpose) = match &element.content {
                ElementContent::Table(t) => (&t.name, &t.purpose),
                ElementContent::Screen(s) => (&s.name, &s.purpose),
            };
            let pinned = matches.pinned.get(&element.id);
            let rank = if element.id.to_lowercase() == lowered || name.to_lowercase() == lowered {
                Some(Rank::Exact)
            } else if pinned
                .is_some_and(|files| files.iter().any(|f| matches.path_paths.contains(f)))
            {
                Some(Rank::Path)
            } else if parts.contains(&element.part_id) {
                Some(Rank::Part)
            } else if !words.is_empty() && {
                let (name, purpose) = (name.to_lowercase(), purpose.to_lowercase());
                let holds = |text: &str| words.iter().all(|w| text.contains(w.as_str()));
                holds(&name) || holds(&purpose)
            } {
                Some(Rank::Words)
            } else {
                None
            };
            if let Some(rank) = rank {
                ranked.push((
                    rank,
                    1,
                    element.id.clone(),
                    number(&element.id),
                    Ranked::Element { rank, index },
                ));
            }
        }
        ranked.sort_by(|a, b| (a.0, a.1, a.3, &a.2).cmp(&(b.0, b.1, b.3, &b.2)));
        let more = ranked.len().saturating_sub(ENTRIES_MAX);
        let entries = ranked
            .into_iter()
            .take(ENTRIES_MAX)
            .map(|(.., entry)| entry)
            .collect();
        (entries, u32::try_from(more).unwrap_or(u32::MAX))
    }
}

/// An element id's number: `E12` is 12; an id without digits sorts last.
fn number(id: &str) -> u64 {
    id.chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use core_design::{
        DesignElement, ElementContent, LayoutPart, PartId, ProjectLayout, TableContent,
    };
    use core_projects::ProjectId;

    use super::{ENTRIES_MAX, Matches, Ranked, Ranking};
    use crate::Rank;

    fn layout() -> ProjectLayout {
        ProjectLayout {
            map_revision: 1,
            standards_base: 1,
            standards_additions: None,
            parts: vec![LayoutPart {
                id: PartId::from_recorded("db".into()),
                parent: None,
                title: "Database".into(),
                kind: None,
                folder: "db".into(),
                grouping: false,
                layers: vec![],
                rules: vec![],
            }],
        }
    }

    fn element(id: &str, name: &str, purpose: &str) -> DesignElement {
        DesignElement {
            id: id.into(),
            revision: 1,
            project_id: ProjectId::from_recorded("p".into()),
            part_id: PartId::from_recorded("db".into()),
            outcomes: vec![],
            content: ElementContent::Table(TableContent {
                name: name.into(),
                purpose: purpose.into(),
                columns: vec![],
            }),
            deleted: false,
            written_by: "t".into(),
            created_at: "now".into(),
        }
    }

    fn file(rank: Rank, path: &str) -> Ranked {
        Ranked::File {
            rank,
            path: path.into(),
        }
    }

    #[test]
    fn an_exact_name_ranks_before_a_path_and_a_path_before_words() {
        let matches = Matches {
            symbol_paths: vec!["z/login.py".into()],
            path_paths: vec!["a/login.py".into()],
            word_paths: vec!["0/notes.py".into()],
            ..Matches::default()
        };
        let (entries, more) = Ranking::entries("login", &layout(), &[], &matches);
        assert_eq!(
            entries,
            vec![
                file(Rank::Exact, "z/login.py"),
                file(Rank::Path, "a/login.py"),
                file(Rank::Words, "0/notes.py"),
            ]
        );
        assert_eq!(more, 0);
    }

    #[test]
    fn an_entry_appears_once_at_its_best_tier() {
        let matches = Matches {
            symbol_paths: vec!["users.py".into()],
            path_paths: vec!["users.py".into()],
            word_paths: vec!["users.py".into()],
            ..Matches::default()
        };
        let users = [element("E1", "users", "every user")];
        let (entries, _) = Ranking::entries("users", &layout(), &users, &matches);
        assert_eq!(
            entries,
            vec![
                file(Rank::Exact, "users.py"),
                Ranked::Element {
                    rank: Rank::Exact,
                    index: 0
                },
            ]
        );
    }

    #[test]
    fn ties_go_by_path_then_element_number() {
        let matches = Matches {
            part_paths: vec!["db/b.sql".into(), "db/a.sql".into()],
            ..Matches::default()
        };
        let elements = [element("E10", "x", "y"), element("E2", "x", "y")];
        let (entries, _) = Ranking::entries("database", &layout(), &elements, &matches);
        assert_eq!(
            entries,
            vec![
                file(Rank::Part, "db/a.sql"),
                file(Rank::Part, "db/b.sql"),
                Ranked::Element {
                    rank: Rank::Part,
                    index: 1
                },
                Ranked::Element {
                    rank: Rank::Part,
                    index: 0
                },
            ]
        );
    }

    #[test]
    fn short_words_and_punctuation_are_not_words() {
        assert!(Ranking::words("a b").is_empty());
        assert_eq!(Ranking::words("user-login"), vec!["user", "login"]);
        let elements = [element("E1", "notes", "a b c")];
        let (entries, _) = Ranking::entries("a b", &layout(), &elements, &Matches::default());
        assert!(entries.is_empty());
    }

    #[test]
    fn twenty_entries_then_a_count() {
        let matches = Matches {
            word_paths: (0..25).map(|n| format!("f{n:02}.py")).collect(),
            ..Matches::default()
        };
        let (entries, more) = Ranking::entries("files", &layout(), &[], &matches);
        assert_eq!(entries.len(), ENTRIES_MAX);
        assert_eq!(more, 5);
        assert_eq!(entries[0], file(Rank::Words, "f00.py"));
    }
}

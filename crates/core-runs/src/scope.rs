//! One job: matching file paths against a task's `writes` patterns.
//!
//! The overlap check at Start and the commit read the same patterns, so both
//! use this one matcher: `*` stops at `/`, `**` crosses it.

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};

/// A set of patterns, compiled once.
pub struct Scope {
    set: GlobSet,
}

impl Scope {
    /// An invalid pattern is an error. No patterns admit nothing.
    pub fn new(patterns: &[String]) -> Result<Self, String> {
        let mut set = GlobSetBuilder::new();
        for pattern in patterns {
            let glob: Glob = GlobBuilder::new(&Self::normal(pattern))
                .literal_separator(true)
                .build()
                .map_err(|error| format!("{pattern}: {error}"))?;
            set.add(glob);
        }
        let set = set.build().map_err(|error| error.to_string())?;
        Ok(Self { set })
    }

    /// Whether `path` (relative to the task's folder) is inside the patterns.
    pub fn admits(&self, path: &str) -> bool {
        self.set.is_match(Self::normal(path))
    }

    /// Whether some path may match a pattern of both sets. Two patterns may
    /// intersect unless their literal prefixes are disjoint; unproven means
    /// they intersect.
    pub fn may_overlap(ours: &[String], theirs: &[String]) -> bool {
        ours.iter().any(|a| {
            theirs
                .iter()
                .any(|b| Self::prefixes_meet(&Self::prefix(a), &Self::prefix(b)))
        })
    }

    fn normal(text: &str) -> String {
        text.replace('\\', "/")
    }

    /// The pattern cut before its first glob character, then at its last `/`.
    /// A pattern with no glob character names one path and is kept whole.
    fn prefix(pattern: &str) -> String {
        let pattern = Self::normal(pattern);
        let pattern = pattern.trim_start_matches("./");
        match pattern.find(['*', '?', '[', '{']) {
            None => pattern.trim_end_matches('/').to_string(),
            Some(at) => match pattern[..at].rfind('/') {
                Some(slash) => pattern[..slash].to_string(),
                None => String::new(),
            },
        }
    }

    /// Whether one prefix is a path prefix of the other.
    fn prefixes_meet(a: &str, b: &str) -> bool {
        let inside = |outer: &str, inner: &str| {
            outer.is_empty() || inner == outer || inner.starts_with(&format!("{outer}/"))
        };
        inside(a, b) || inside(b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::Scope;

    fn all(patterns: &[&str]) -> Vec<String> {
        patterns.iter().map(|p| p.to_string()).collect()
    }

    fn scope(patterns: &[&str]) -> Scope {
        Scope::new(&all(patterns)).unwrap()
    }

    #[test]
    fn a_star_does_not_cross_a_separator() {
        let scope = scope(&["backend/*.rs"]);
        assert!(scope.admits("backend/a.rs"));
        assert!(!scope.admits("backend/deep/a.rs"));
    }

    #[test]
    fn a_double_star_crosses_separators() {
        assert!(scope(&["backend/**"]).admits("backend/deep/a.rs"));
    }

    #[test]
    fn no_patterns_admit_nothing() {
        assert!(!scope(&[]).admits("a.rs"));
    }

    #[test]
    fn an_invalid_pattern_is_an_error() {
        assert!(Scope::new(&all(&["backend/[a"])).is_err());
    }

    #[test]
    fn windows_separators_are_normalised() {
        let scope = scope(&["backend\\app\\**"]);
        assert!(scope.admits("backend\\app\\x.php"));
        assert!(scope.admits("backend/app/x.php"));
    }

    #[test]
    fn a_folder_and_its_subfolder_may_overlap() {
        assert!(Scope::may_overlap(
            &all(&["backend/app/**"]),
            &all(&["backend/app/Models/**"])
        ));
    }

    #[test]
    fn disjoint_folders_do_not_overlap() {
        assert!(!Scope::may_overlap(
            &all(&["backend/app/**"]),
            &all(&["web/**"])
        ));
        assert!(!Scope::may_overlap(
            &all(&["src/a.rs"]),
            &all(&["src/b.rs"])
        ));
    }

    #[test]
    fn a_pattern_without_a_folder_may_overlap_anything() {
        assert!(Scope::may_overlap(&all(&["*.md"]), &all(&["web/**"])));
    }
}

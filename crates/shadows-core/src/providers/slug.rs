//! The shape of a generated configuration slug, before its owner checks it is free.

/// A lowercase ASCII base of at most 29 characters that starts with a letter.
pub(crate) struct SlugBase(String);

impl SlugBase {
    pub(crate) fn new(name: &str, fallback: &str) -> Self {
        let mut base = String::new();
        for c in name.to_ascii_lowercase().chars() {
            if c.is_ascii_alphanumeric() {
                base.push(c);
            } else if !base.is_empty() && !base.ends_with('-') {
                base.push('-');
            }
        }
        base = base.trim_matches('-').into();
        if base.is_empty() {
            base = fallback.into();
        }
        if !base.starts_with(|c: char| c.is_ascii_lowercase()) {
            base = format!("{fallback}-{base}");
        }
        base.truncate(29);
        Self(base)
    }

    /// The base itself at `n` 1, then the base with `-n`, within 32 characters.
    pub(crate) fn candidate(&self, n: usize) -> String {
        if n <= 1 {
            return self.0.clone();
        }
        let suffix = format!("-{n}");
        format!(
            "{}{}",
            &self.0[..self.0.len().min(32 - suffix.len())],
            suffix
        )
    }
}

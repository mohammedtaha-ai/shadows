//! One job: the text a title taken from a conversation may read (spec §4.2).
//!
//! A title is one line: whitespace collapsed, at most `TITLE_MAX_CHARS`
//! characters, cut with an ellipsis. An empty one is no title.

/// About one sidebar line.
pub(crate) const TITLE_MAX_CHARS: usize = 60;

/// The first non-blank line of a thread's first message, as a title.
pub(crate) fn from_first_message(message: &str) -> Option<String> {
    one_line(message.lines().find(|l| !l.trim().is_empty())?)
}

/// A title the harness generated, as Shadows keeps it. A placeholder is no
/// title: Claude Code's generator answers "New session" for a conversation too
/// short to name ("hi"), and that must not replace the first message.
pub(crate) fn from_harness(title: &str) -> Option<String> {
    one_line(title).filter(|t| !is_placeholder(t))
}

/// The generic names a harness gives a session it could not name.
fn is_placeholder(title: &str) -> bool {
    const PLACEHOLDERS: [&str; 4] = ["new session", "new conversation", "untitled", "new chat"];
    let lowered = title.trim_end_matches(['.', '!']).to_lowercase();
    PLACEHOLDERS.contains(&lowered.as_str())
}

fn one_line(text: &str) -> Option<String> {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    if collapsed.chars().count() <= TITLE_MAX_CHARS {
        return Some(collapsed);
    }
    let cut: String = collapsed.chars().take(TITLE_MAX_CHARS - 1).collect();
    Some(format!("{}…", cut.trim_end()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placeholder_from_the_harness_is_no_title() {
        assert_eq!(from_harness("New session"), None);
        assert_eq!(from_harness("  new   Session. "), None);
        assert_eq!(
            from_harness("New session handling in Shadows").as_deref(),
            Some("New session handling in Shadows")
        );
    }
}

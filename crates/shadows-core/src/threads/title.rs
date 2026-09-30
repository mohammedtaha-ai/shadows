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

/// A title the harness generated, as Shadows keeps it.
pub(crate) fn from_harness(title: &str) -> Option<String> {
    one_line(title)
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

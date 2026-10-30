//! One job: the modes Shadows allows per harness — a decision of
//! Shadows, not a list of what the harness offers.
//!
//! What a harness offers comes from its session (`choices.rs`); this file only
//! says which of those offered modes Shadows lets a turn use at all, how each
//! is named and described, which one is unattended, and which harnesses
//! exist. A mode reaches a menu only if it is here, the harness
//! lists it, and the project allows it.

use std::collections::BTreeMap;

/// Claude Code, run through the pinned ACP adapter.
pub const CLAUDE_CODE: &str = "claude-code";
/// Codex: listed, not yet runnable.
pub const CODEX: &str = "codex";

/// AYLUL's brain: Claude Code on its own profile, opened without Planner setup.
pub const AYLUL: &str = "aylul";

/// Every harness Shadows knows, in the order a client lists them.
pub const KNOWN: [&str; 3] = [CLAUDE_CODE, CODEX, AYLUL];

/// An allowed mode with the words a client shows for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

// Claude Code's own acceptEdits runs file commands inside the project folder
// without a permission request, so Shadows never sees them; anything else is
// asked, and Shadows refuses it.
const CLAUDE_CODE_MODES: [Mode; 2] = [
    Mode {
        id: "acceptEdits",
        label: "Accept edits",
        description: "Claude Code edits, creates and deletes files in the project folder \
                      without asking. Other commands are refused.",
    },
    Mode {
        id: "auto",
        label: "Auto",
        description: "Claude Code decides on its own; nothing is asked.",
    },
];

// The ids are read out of the table above, so a mode is named once.
const CLAUDE_CODE_MODE_IDS: [&str; CLAUDE_CODE_MODES.len()] = ids(&CLAUDE_CODE_MODES);

const fn ids<const N: usize>(modes: &[Mode; N]) -> [&'static str; N] {
    let mut out = [""; N];
    let mut i = 0;
    while i < N {
        out[i] = modes[i].id;
        i += 1;
    }
    out
}

/// The allowed modes of `kind` with their labels and descriptions, in the
/// order a client lists them.
pub fn modes(kind: &str) -> &'static [Mode] {
    match kind {
        CLAUDE_CODE | AYLUL => &CLAUDE_CODE_MODES,
        _ => &[],
    }
}

/// The ids of `modes(kind)`, in the same order. Empty for a harness whose
/// modes are not decided (Codex) or that Shadows does not know.
pub fn allowed_modes(kind: &str) -> &'static [&'static str] {
    match kind {
        CLAUDE_CODE | AYLUL => &CLAUDE_CODE_MODE_IDS,
        _ => &[],
    }
}

/// The allowed mode that asks nothing: the one that would let a refused
/// permission through.
pub fn unattended_mode(kind: &str) -> Option<&'static str> {
    match kind {
        CLAUDE_CODE | AYLUL => Some("auto"),
        _ => None,
    }
}

/// The mode a new conversation on `kind` starts in, also used when its
/// remembered mode is disallowed.
pub fn default_mode(kind: &str) -> Option<&'static str> {
    match kind {
        CLAUDE_CODE | AYLUL => Some("acceptEdits"),
        _ => None,
    }
}

pub fn is_known(kind: &str) -> bool {
    KNOWN.contains(&kind)
}

/// Whether a turn can run on `kind` in this daemon. Claude Code's three paths
/// are required flags, so a running daemon always has them.
pub fn is_available(kind: &str) -> bool {
    kind == CLAUDE_CODE
}

/// `is_available`, plus AYLUL when the daemon has an `aylul` profile.
pub fn is_available_with(kind: &str, aylul_configured: bool) -> bool {
    is_available(kind) || (kind == AYLUL && aylul_configured)
}

/// What a new project allows: every mode of every harness's policy.
pub fn default_modes() -> BTreeMap<String, Vec<String>> {
    KNOWN
        .iter()
        .filter(|kind| !allowed_modes(kind).is_empty())
        .map(|kind| {
            let modes = allowed_modes(kind).iter().map(|m| m.to_string()).collect();
            (kind.to_string(), modes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aylul_is_known_with_modes() {
        assert!(is_known(AYLUL));
        assert_eq!(allowed_modes(AYLUL), ["acceptEdits", "auto"]);
        assert_eq!(default_mode(AYLUL), Some("acceptEdits"));
        assert!(!is_available(AYLUL));
        assert!(!is_available_with(AYLUL, false));
        assert!(is_available_with(AYLUL, true));
    }

    #[test]
    fn the_initial_and_unattended_modes_are_allowed() {
        for kind in KNOWN {
            let ids: Vec<&str> = modes(kind).iter().map(|m| m.id).collect();
            assert_eq!(ids, allowed_modes(kind), "{kind}");
            let wanted = [default_mode(kind), unattended_mode(kind)];
            for id in wanted.into_iter().flatten() {
                assert!(ids.contains(&id), "{kind} {id}");
            }
        }
        assert_eq!(unattended_mode(AYLUL), Some("auto"));
        assert!(modes(CODEX).is_empty());
        assert_eq!(unattended_mode(CODEX), None);
    }
}

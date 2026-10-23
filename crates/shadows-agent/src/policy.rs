//! One job: the modes Shadows allows per harness (spec §12.4) — a decision of
//! Shadows, not a list of what the harness offers.
//!
//! What a harness offers comes from its session (`choices.rs`); this file only
//! says which of those offered modes Shadows lets a turn use at all, and which
//! harnesses exist. A mode reaches a menu only if it is here, the harness
//! lists it, and the project allows it (§12.5).

use std::collections::BTreeMap;

/// Claude Code, run through the pinned ACP adapter (§12.2).
pub const CLAUDE_CODE: &str = "claude-code";
/// Codex: listed, not yet runnable (§12.1).
pub const CODEX: &str = "codex";

/// AYLUL's brain: Claude Code on its own profile, opened without Planner setup.
pub const AYLUL: &str = "aylul";

/// Every harness Shadows knows, in the order a client lists them.
pub const KNOWN: [&str; 3] = [CLAUDE_CODE, CODEX, AYLUL];

/// The modes Shadows allows for `kind`. Empty for a harness whose modes are
/// not decided (Codex) or that Shadows does not know.
pub fn allowed_modes(kind: &str) -> &'static [&'static str] {
    match kind {
        CLAUDE_CODE | AYLUL => &["acceptEdits", "auto"],
        _ => &[],
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
/// are required flags, so a running daemon always has them (§12.4).
pub fn is_available(kind: &str) -> bool {
    kind == CLAUDE_CODE
}

/// `is_available`, plus AYLUL when the daemon has an `aylul` profile.
pub fn is_available_with(kind: &str, aylul_configured: bool) -> bool {
    is_available(kind) || (kind == AYLUL && aylul_configured)
}

/// What a new project allows: every mode of every harness's policy (§12.5).
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
}

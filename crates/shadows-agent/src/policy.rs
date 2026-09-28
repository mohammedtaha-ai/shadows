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

/// Every harness Shadows knows, in the order a client lists them.
pub const KNOWN: [&str; 2] = [CLAUDE_CODE, CODEX];

/// The modes Shadows allows for `kind`. Empty for a harness whose modes are
/// not decided (Codex) or that Shadows does not know.
pub fn allowed_modes(kind: &str) -> &'static [&'static str] {
    match kind {
        CLAUDE_CODE => &["acceptEdits", "auto"],
        _ => &[],
    }
}

/// The mode every new conversation on `kind` starts in, and every opening of
/// its session is set to (§12.2). Never remembered.
pub fn default_mode(kind: &str) -> Option<&'static str> {
    match kind {
        CLAUDE_CODE => Some("acceptEdits"),
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

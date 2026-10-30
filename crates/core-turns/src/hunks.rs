//! Place and count the hunks reported by the harness.
use std::collections::HashMap;

use shadows_agent::events::{DiffHunk, DiffLine, DiffOp, ToolDiff, ToolLocation};
use similar::{ChangeTag, TextDiff};

pub(crate) struct Placed {
    pub hunks: Vec<DiffHunk>,
    pub added: u32,
    pub removed: u32,
}

pub(crate) struct Hunks;

impl Hunks {
    pub fn from_diffs(diffs: &[ToolDiff], locations: &[ToolLocation]) -> Placed {
        let mut result = Placed {
            hunks: Vec::new(),
            added: 0,
            removed: 0,
        };
        let mut positions: HashMap<&str, usize> = HashMap::new();
        let mut net: HashMap<&str, i64> = HashMap::new();
        for diff in diffs {
            let ordinal = positions.entry(&diff.path).or_default();
            let new_start = locations
                .iter()
                .filter(|loc| loc.path == diff.path)
                .nth(*ordinal)
                .and_then(|loc| loc.line);
            *ordinal += 1;
            let old_start = new_start.and_then(|n| {
                u32::try_from(i64::from(n) - net.get(diff.path.as_str()).copied().unwrap_or(0)).ok()
            });
            let mut lines = Vec::new();
            let old = diff.old_text.as_deref().unwrap_or("");
            let patch = TextDiff::from_lines(old, &diff.new_text);
            for change in patch.iter_all_changes() {
                let op = match change.tag() {
                    ChangeTag::Equal => DiffOp::Context,
                    ChangeTag::Insert => {
                        result.added += 1;
                        DiffOp::Add
                    }
                    ChangeTag::Delete => {
                        result.removed += 1;
                        DiffOp::Remove
                    }
                };
                let delta = match op {
                    DiffOp::Add => 1,
                    DiffOp::Remove => -1,
                    _ => 0,
                };
                *net.entry(&diff.path).or_default() += delta;
                lines.push(DiffLine {
                    op,
                    text: change
                        .value()
                        .trim_end_matches('\n')
                        .trim_end_matches('\r')
                        .to_owned(),
                });
            }
            result.hunks.push(DiffHunk {
                path: diff.path.clone(),
                old_start,
                new_start,
                lines,
            });
        }
        result
    }
}

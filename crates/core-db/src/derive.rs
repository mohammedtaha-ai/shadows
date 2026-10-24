//! Command ids derived from stable request anchors when a caller names none.
//!
//! A model is not asked to invent an id or to reuse one on a retry, so the id
//! is derived from what makes a retry the same request. Every derived id but a
//! `draft_ref` carries the fingerprint: the same request repeated is a replay,
//! a different one is a different command. A `draft_ref` names one intended
//! plan, so the same ref with different arguments is a `CommandConflict`.

/// What a derived id is anchored to.
#[derive(Debug, Clone, Copy)]
pub enum Anchor<'a> {
    /// A write carrying `expected_revision`.
    Revision(i64),
    /// `plan_show` or `draft_start` by the internal Planner: its running turn.
    Operation(&'a str),
    /// `draft_start` by an external agent: the ref `draft_prepare` issued.
    DraftRef(&'a str),
}

impl Anchor<'_> {
    /// `rev:7:<16 hex>`, `op:<operation id>:<16 hex>` or `ref:<draft_ref>`, the
    /// hex being the first sixteen of the request fingerprint.
    pub fn derived_id(self, fingerprint: &str) -> String {
        let short = fingerprint.get(..16).unwrap_or(fingerprint);
        match self {
            Anchor::Revision(n) => format!("rev:{n}:{short}"),
            Anchor::Operation(op) => format!("op:{op}:{short}"),
            Anchor::DraftRef(r) => format!("ref:{r}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Anchor;

    #[test]
    fn derived_ids_match_persisted_values() {
        let fingerprint = "c2daffdac71b1da272da47656f5a77a5fab0bc3b1890ac2d292b99e1cf4b7ad8";
        for (anchor, expected) in [
            (Anchor::Revision(7), "rev:7:c2daffdac71b1da2"),
            (Anchor::Operation("op-1"), "op:op-1:c2daffdac71b1da2"),
            (Anchor::DraftRef("dr-1"), "ref:dr-1"),
        ] {
            assert_eq!(anchor.derived_id(fingerprint), expected);
        }
        assert_eq!(Anchor::Revision(-1).derived_id("abc"), "rev:-1:abc");
        assert_eq!(Anchor::Operation("op:é").derived_id(""), "op:op:é:");
    }
}

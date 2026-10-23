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

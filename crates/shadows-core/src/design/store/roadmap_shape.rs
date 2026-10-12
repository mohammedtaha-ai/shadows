//! Checking a proposed roadmap against its action and the parts and outcomes it names.
use super::outcomes::load as load_outcome;
use crate::design::{OutcomeContent, OutcomeId, RoadmapAction, RoadmapPropose};
use crate::{ProjectId, StorageError};
use sqlx::SqliteConnection;
use std::collections::{BTreeMap, BTreeSet};

fn bad(message: &str) -> StorageError {
    StorageError::Constraint(message.into())
}

/// Shape rules that need no database: counts, names, keys, parents and parts per action.
pub(super) fn validate(args: &mut RoadmapPropose) -> Result<(), StorageError> {
    let count_ok = match args.action {
        RoadmapAction::First => !args.outcomes.is_empty(),
        RoadmapAction::Add | RoadmapAction::Change => args.outcomes.len() == 1,
        RoadmapAction::Remove => args.outcomes.is_empty(),
    };
    if !count_ok {
        return Err(bad(
            "first needs outcomes; add/change needs exactly one; remove needs none",
        ));
    }
    let names = matches!(args.action, RoadmapAction::Change | RoadmapAction::Remove);
    if names != args.outcome_id.is_some() {
        return Err(bad("only a change or remove names outcome_id"));
    }
    if args
        .outcome_id
        .as_ref()
        .is_some_and(|id| uuid::Uuid::parse_str(id.as_str()).is_err())
    {
        return Err(bad("outcome_id must be a UUID"));
    }
    let mut keys = BTreeSet::new();
    for outcome in &mut args.outcomes {
        outcome.content.title = outcome.content.title.trim().to_owned();
        if outcome.content.title.is_empty() {
            return Err(bad("an outcome needs a title"));
        }
        if outcome.parts.is_empty() {
            return Err(bad("an outcome references at least one part"));
        }
        let slug = !outcome.key.is_empty()
            && outcome
                .key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !slug || !keys.insert(outcome.key.clone()) {
            return Err(bad("outcome keys are unique slugs"));
        }
        match args.action {
            RoadmapAction::First if outcome.parent.is_some() => {
                return Err(bad("a first roadmap names parent_key only"));
            }
            RoadmapAction::Add if outcome.parent_key.is_some() => {
                return Err(bad("an add names an existing parent, not parent_key"));
            }
            RoadmapAction::Change if outcome.parent.is_some() || outcome.parent_key.is_some() => {
                return Err(bad(
                    "a change edits content; use the page to move an outcome",
                ));
            }
            _ => {}
        }
    }
    check_parent_keys(args)
}

/// Every parent_key names another outcome of the proposal, and no chain loops.
fn check_parent_keys(args: &RoadmapPropose) -> Result<(), StorageError> {
    let parents: BTreeMap<_, _> = args
        .outcomes
        .iter()
        .map(|o| (o.key.as_str(), o.parent_key.as_deref()))
        .collect();
    for outcome in &args.outcomes {
        let mut at = outcome.parent_key.as_deref();
        for _ in 0..=parents.len() {
            match at {
                None => break,
                Some(key) => match parents.get(key) {
                    None => return Err(bad("parent_key names no outcome of this proposal")),
                    Some(_) if key == outcome.key => return Err(bad("outcome containment cycle")),
                    Some(next) => at = *next,
                },
            }
        }
        if at.is_some() {
            return Err(bad("outcome containment cycle"));
        }
    }
    Ok(())
}

/// A roadmap is proposed and approved only once the structure write is complete (§23.6).
pub(super) async fn require_structure(
    conn: &mut SqliteConnection,
    project: &ProjectId,
) -> Result<(), StorageError> {
    let written: Option<String> =
        sqlx::query_scalar("SELECT completed_at FROM design_structure_write WHERE project_id=?")
            .bind(project.as_str())
            .fetch_optional(&mut *conn)
            .await?
            .flatten();
    if written.is_none() {
        return Err(StorageError::StageBlocked(
            "structure not written: press Create structure first".into(),
        ));
    }
    Ok(())
}

/// The part revisions at proposal time; an unknown part refuses the proposal.
pub(super) async fn part_revisions(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    args: &RoadmapPropose,
) -> Result<BTreeMap<String, i64>, StorageError> {
    let mut revisions = BTreeMap::new();
    for part in args.outcomes.iter().flat_map(|o| &o.parts) {
        let revision: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM design_part WHERE project_id=? AND id=?")
                .bind(project.as_str())
                .bind(part.as_str())
                .fetch_optional(&mut *conn)
                .await?;
        let revision = revision.ok_or_else(|| {
            StorageError::Constraint(format!("part {part} does not belong to project {project}"))
        })?;
        revisions.insert(part.to_string(), revision);
    }
    Ok(revisions)
}

/// The outcome a change or remove names, and the existing parent an add names.
pub(super) async fn existing(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    args: &RoadmapPropose,
) -> Result<Option<(OutcomeContent, i64)>, StorageError> {
    let unknown = |id: &OutcomeId| {
        StorageError::Constraint(format!("outcome {id} does not belong to project {project}"))
    };
    if let Some(parent) = args.outcomes.iter().find_map(|o| o.parent.as_ref()) {
        load_outcome(conn, project, parent)
            .await
            .map_err(|_| unknown(parent))?;
    }
    let Some(id) = &args.outcome_id else {
        return Ok(None);
    };
    let outcome = load_outcome(conn, project, id)
        .await
        .map_err(|_| unknown(id))?;
    Ok(Some((outcome.content, outcome.revision)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::{PartId, ProposedOutcome};

    fn outcome(key: &str, parent_key: Option<&str>) -> ProposedOutcome {
        ProposedOutcome {
            key: key.into(),
            parent_key: parent_key.map(Into::into),
            parent: None,
            content: OutcomeContent {
                title: format!(" {key} "),
                intended_result: String::new(),
                acceptance: vec![],
            },
            parts: vec![PartId::generate()],
        }
    }

    fn first(outcomes: Vec<ProposedOutcome>) -> RoadmapPropose {
        RoadmapPropose {
            action: RoadmapAction::First,
            outcomes,
            outcome_id: None,
            replaces: None,
        }
    }

    #[test]
    fn a_parent_cycle_is_refused() {
        let mut args = first(vec![outcome("a", Some("b")), outcome("b", Some("a"))]);
        assert!(
            validate(&mut args)
                .unwrap_err()
                .to_string()
                .contains("cycle")
        );
    }

    #[test]
    fn titles_are_trimmed_and_keys_must_be_unique() {
        let mut args = first(vec![outcome("a", None), outcome("b", Some("a"))]);
        validate(&mut args).unwrap();
        assert_eq!(args.outcomes[0].content.title, "a");
        let mut twice = first(vec![outcome("a", None), outcome("a", None)]);
        assert!(validate(&mut twice).is_err());
    }
}

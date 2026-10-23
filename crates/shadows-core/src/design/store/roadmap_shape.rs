//! Checking a proposed roadmap against its action and the parts and outcomes it names.
use super::outcomes::OutcomesRows;
use crate::design::{Outcome, OutcomeId, RoadmapAction, RoadmapPropose};
use crate::{ProjectId, StorageError};
use sqlx::SqliteConnection;
use std::collections::{BTreeMap, BTreeSet};

fn bad(message: &str) -> StorageError {
    StorageError::Constraint(message.into())
}

/// Shape rules that need no database: counts, names, keys, parents and parts per action.
impl RoadmapShapeRows {
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
                RoadmapAction::Change if outcome.parent_key.is_some() => {
                    return Err(bad("a change names its parent outcome, not parent_key"));
                }
                _ => {}
            }
        }
        check_parent_keys(args)
    }
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

/// A roadmap is proposed and approved only once the structure write is complete.
impl RoadmapShapeRows {
    pub(super) async fn require_structure(
        conn: &mut SqliteConnection,
        project: &ProjectId,
    ) -> Result<(), crate::CoreError> {
        let written: Option<String> = sqlx::query_scalar(
            "SELECT completed_at FROM design_structure_write WHERE project_id=?",
        )
        .bind(project.as_str())
        .fetch_optional(&mut *conn)
        .await?
        .flatten();
        if written.is_none() {
            return Err(crate::design::DesignError::StageBlocked(
                "structure not written: press Create structure first".into(),
            )
            .into());
        }
        Ok(())
    }
}

/// Each referenced part's snapshot at proposal time; an unknown part refuses it.
impl RoadmapShapeRows {
    pub(super) async fn part_snapshots(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        args: &RoadmapPropose,
    ) -> Result<BTreeMap<String, String>, StorageError> {
        let mut snapshots = BTreeMap::new();
        for part in args.outcomes.iter().flat_map(|o| &o.parts) {
            let snapshot = RoadmapShapeRows::part_snapshot(conn, project, part.as_str())
                .await?
                .ok_or_else(|| {
                    StorageError::Constraint(format!(
                        "part {part} does not belong to project {project}"
                    ))
                })?;
            snapshots.insert(part.to_string(), snapshot);
        }
        Ok(snapshots)
    }
}

/// What a card depends on in a part: its content. Linking a plan or reordering
/// raises the part's revision but leaves this unchanged.
impl RoadmapShapeRows {
    pub(super) async fn part_snapshot(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        part: &str,
    ) -> Result<Option<String>, StorageError> {
        Ok(
            sqlx::query_scalar("SELECT content_json FROM design_part WHERE project_id=? AND id=?")
                .bind(project.as_str())
                .bind(part)
                .fetch_optional(&mut *conn)
                .await?,
        )
    }
}

/// What a change or remove depends on in its outcome: its parent and content.
impl RoadmapShapeRows {
    pub(super) fn outcome_snapshot(outcome: &Outcome) -> Result<String, StorageError> {
        Ok(serde_json::to_string(&(&outcome.parent, &outcome.content))?)
    }
}

/// The outcome a change or remove names, and the existing parent an outcome names.
impl RoadmapShapeRows {
    pub(super) async fn existing(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        args: &RoadmapPropose,
    ) -> Result<Option<Outcome>, StorageError> {
        let unknown = |id: &OutcomeId| {
            StorageError::Constraint(format!("outcome {id} does not belong to project {project}"))
        };
        if let Some(parent) = args.outcomes.iter().find_map(|o| o.parent.as_ref()) {
            OutcomesRows::load(conn, project, parent)
                .await
                .map_err(|_| unknown(parent))?;
        }
        let Some(id) = &args.outcome_id else {
            return Ok(None);
        };
        let outcome = OutcomesRows::load(conn, project, id)
            .await
            .map_err(|_| unknown(id))?;
        Ok(Some(outcome))
    }
}

pub(in crate::design) struct RoadmapShapeRows;

#[cfg(test)]
mod tests {
    use super::{RoadmapAction, RoadmapPropose, RoadmapShapeRows};
    use crate::design::{OutcomeContent, PartId, ProposedOutcome};

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
            RoadmapShapeRows::validate(&mut args)
                .unwrap_err()
                .to_string()
                .contains("cycle")
        );
    }

    #[test]
    fn titles_are_trimmed_and_keys_must_be_unique() {
        let mut args = first(vec![outcome("a", None), outcome("b", Some("a"))]);
        RoadmapShapeRows::validate(&mut args).unwrap();
        assert_eq!(args.outcomes[0].content.title, "a");
        let mut twice = first(vec![outcome("a", None), outcome("a", None)]);
        assert!(RoadmapShapeRows::validate(&mut twice).is_err());
    }
}

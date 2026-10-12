//! Assemble a proposed map against current parts without writing.
use super::map_versions::{MapRow, contents, views};
use crate::design::{EffectiveStandards, MapAction, MapPropose, map_rules};
use crate::{PartContent, ProjectId, StorageError, VisionContent};

pub(super) fn complete(vision: &VisionContent) -> Result<(), StorageError> {
    let stage = crate::design::stage::compute(vision, &[], &[]);
    if matches!(stage.stage, crate::Stage::Idea | crate::Stage::Vision) {
        let missing = if stage.stage == crate::Stage::Idea {
            "purpose, users, goals, boundaries, technical_direction".into()
        } else {
            stage.missing.join(", ")
        };
        return Err(StorageError::StageBlocked(format!(
            "vision incomplete: {missing}"
        )));
    }
    Ok(())
}

pub(super) fn validate_args(
    args: &mut MapPropose,
    standards: &EffectiveStandards,
) -> Result<(), StorageError> {
    let bad = |s: &str| StorageError::Constraint(s.into());
    if args.parts.is_empty() || (args.action != MapAction::First && args.parts.len() != 1) {
        return Err(bad("first needs parts; add/change needs exactly one part"));
    }
    if (args.action == MapAction::Change) != args.part_id.is_some() {
        return Err(bad("only a change names part_id"));
    }
    if args
        .part_id
        .as_ref()
        .is_some_and(|id| uuid::Uuid::parse_str(id.as_str()).is_err())
    {
        return Err(bad("part_id must be a UUID"));
    }
    let mut waivers = std::collections::BTreeSet::new();
    for waiver in &args.waivers {
        if args.action != MapAction::First
            || !waivers.insert(&waiver.part)
            || !standards
                .base
                .parts
                .iter()
                .any(|p| p.name == waiver.part && p.waivable)
            || waiver.reason.trim().is_empty()
        {
            return Err(bad(
                "a first-map waiver needs a unique waivable kind and a reason",
            ));
        }
    }
    for part in &mut args.parts {
        map_rules::normalise(&mut part.content).map_err(StorageError::Constraint)?;
        match args.action {
            MapAction::First
                if part.parent.is_some()
                    || part.key.is_empty()
                    || !part
                        .key
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
            {
                return Err(bad("first-map parts need slug keys and parent_key only"));
            }
            MapAction::Add if part.parent_key.is_some() => {
                return Err(bad("an add names an existing parent, not parent_key"));
            }
            MapAction::Change if part.parent.is_some() || part.parent_key.is_some() => {
                return Err(bad("a change edits content; use the page to move a part"));
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn result(args: &MapPropose, old: &[MapRow]) -> Result<Vec<MapRow>, StorageError> {
    let mut rows = old
        .iter()
        .map(|r| MapRow {
            id: r.id.clone(),
            parent_id: r.parent_id.clone(),
            revision: r.revision,
            ordinal: r.ordinal,
            content_json: r.content_json.clone(),
            served_revisions: r.served_revisions.clone(),
        })
        .collect::<Vec<_>>();
    if args.action == MapAction::First {
        rows.clear();
    }
    for (n, p) in args.parts.iter().enumerate() {
        match args.action {
            MapAction::Change => {
                let id = args.part_id.as_ref().expect("validated change");
                let row = rows
                    .iter_mut()
                    .find(|r| r.id == id.as_str())
                    .ok_or(StorageError::NotFound("part"))?;
                row.content_json = serde_json::to_string(&p.content)?;
            }
            MapAction::First | MapAction::Add => rows.push(MapRow {
                id: if args.action == MapAction::First {
                    p.key.clone()
                } else {
                    "new-part".into()
                },
                parent_id: if args.action == MapAction::First {
                    p.parent_key.clone()
                } else {
                    p.parent.as_ref().map(ToString::to_string)
                },
                ordinal: n as i64,
                revision: 1,
                content_json: serde_json::to_string(&p.content)?,
                served_revisions: "{}".into(),
            }),
        }
    }
    Ok(rows)
}

pub(super) fn written(args: &MapPropose) -> Vec<&str> {
    match args.action {
        MapAction::First => args.parts.iter().map(|p| p.key.as_str()).collect(),
        MapAction::Add => vec!["new-part"],
        MapAction::Change => vec![args.part_id.as_ref().expect("validated change").as_str()],
    }
}

pub(super) fn check(
    args: &MapPropose,
    rows: &[MapRow],
    vision: &VisionContent,
    standards: &EffectiveStandards,
) -> Result<(), StorageError> {
    let c = contents(rows)?;
    map_rules::check_shape(&views(rows, &c), &written(args), vision, standards)
        .map_err(StorageError::Constraint)
}

pub(super) async fn pinned(
    conn: &mut sqlx::SqliteConnection,
    p: &ProjectId,
    turn: &crate::OperationId,
) -> Result<EffectiveStandards, StorageError> {
    let (base, id): (i64, Option<String>) = sqlx::query_as(
        "SELECT standards_version,standards_additions_version_id FROM agent_invocation
         WHERE operation_id=?",
    )
    .bind(turn.as_str())
    .fetch_one(&mut *conn)
    .await?;
    let mut standards = super::map_versions::effective(conn, p).await?;
    standards.base.version = base;
    standards.additions = match id {
        None => None,
        Some(id) => Some(super::standards::load_version(conn, &id).await?),
    };
    Ok(standards)
}

pub(super) fn old_content(
    args: &MapPropose,
    rows: &[MapRow],
) -> Result<Option<PartContent>, StorageError> {
    args.part_id
        .as_ref()
        .map(|id| {
            rows.iter()
                .find(|r| r.id == id.as_str())
                .ok_or(StorageError::NotFound("part"))
                .and_then(|r| serde_json::from_str(&r.content_json).map_err(Into::into))
        })
        .transpose()
}

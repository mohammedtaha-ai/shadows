//! Persistence of table and screen revisions.
use std::collections::HashSet;

use core_db::{CommandContext, JournalEntry, Storage, StorageError};
use core_grants::Writer;
use core_projects::Projects;
use sqlx::{FromRow, SqliteConnection};

use super::proposal_origin::ProposalOrigin;
use super::{DesignStore, outcomes::OutcomesRows};
use crate::elements::{ElementRules, ElementWrite};
use crate::{
    Design, DesignElement, DesignError, ElementContent, ElementDelete, ElementInput, ElementKind,
    OperationId, PartId, ProjectId, ScreenContent, TableContent,
};

const LIVE_LIMIT: usize = 500;

#[derive(FromRow)]
struct ElementRow {
    element_id: String,
    revision: i64,
    part_id: String,
    outcomes_json: String,
    content_json: String,
    deleted: bool,
    written_by: String,
    created_at: String,
}

impl ElementRow {
    fn view(self, project: &ProjectId) -> Result<DesignElement, DesignError> {
        Ok(DesignElement {
            id: self.element_id,
            revision: self.revision,
            project_id: project.clone(),
            part_id: PartId::from_stored(self.part_id),
            outcomes: serde_json::from_str(&self.outcomes_json)?,
            content: serde_json::from_str(&self.content_json)?,
            deleted: self.deleted,
            written_by: self.written_by,
            created_at: self.created_at,
        })
    }
}

impl DesignStore {
    pub(crate) async fn write_element_in(
        &self,
        ctx: CommandContext,
        writer: Writer,
        project: ProjectId,
        turn: Option<OperationId>,
        write: ElementWrite,
    ) -> Result<DesignElement, DesignError> {
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    let origin =
                        ProposalOrigin::new(conn, &writer, &project, turn.as_ref()).await?;
                    if let Some(at) =
                        CommandContext::classify(conn, &ctx, "Project", project.as_str()).await?
                    {
                        return recorded(conn, &project, &at).await;
                    }
                    origin.require_running(conn).await?;
                    Design::require_roadmap_in(conn, &project).await?;
                    let ts = Storage::now();
                    let element = match write {
                        ElementWrite::Put(input) => {
                            put_element_in(conn, &project, input, &ctx.principal_id, &ts).await?
                        }
                        ElementWrite::Delete(input) => {
                            delete_element_in(conn, &project, input, &ctx.principal_id, &ts).await?
                        }
                    };
                    insert(conn, &element).await?;
                    let event =
                        origin
                            .journal("DesignElementChanged")
                            .with_payload(serde_json::json!({
                                "project_id": project,
                                "element_id": element.id,
                                "revision": element.revision,
                            }));
                    JournalEntry::append_event(conn, event, &ts).await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Project",
                        project.as_str(),
                        "Element",
                        &format!("{}@{}", element.id, element.revision),
                        &ts,
                    )
                    .await?;
                    Ok(element)
                })
            })
            .await
    }

    pub(crate) async fn elements(
        &self,
        project: &ProjectId,
    ) -> Result<Vec<DesignElement>, DesignError> {
        let mut tx = self.storage.reader().begin().await?;
        Projects::require_live_in(&mut tx, project).await?;
        let result = latest_elements(&mut tx, project, true).await?;
        tx.commit().await?;
        Ok(result)
    }

    pub(crate) async fn element_revisions(
        &self,
        project: &ProjectId,
        id: &str,
    ) -> Result<Vec<DesignElement>, DesignError> {
        let mut tx = self.storage.reader().begin().await?;
        Projects::require_live_in(&mut tx, project).await?;
        let rows: Vec<ElementRow> = sqlx::query_as(
            "SELECT element_id, revision, part_id, outcomes_json, content_json, deleted,
                    written_by, created_at
               FROM design_elements WHERE project_id=? AND element_id=?
              ORDER BY revision",
        )
        .bind(project.as_str())
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        rows.into_iter().map(|row| row.view(project)).collect()
    }

    pub(crate) async fn element_in(
        conn: &mut SqliteConnection,
        project: &ProjectId,
        id: &str,
        revision: Option<i64>,
    ) -> Result<Option<DesignElement>, DesignError> {
        Projects::require_live_in(conn, project).await?;
        find_revision(conn, project, id, revision).await
    }
}

async fn find_revision(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &str,
    revision: Option<i64>,
) -> Result<Option<DesignElement>, DesignError> {
    let row: Option<ElementRow> = sqlx::query_as(
        "SELECT element_id, revision, part_id, outcomes_json, content_json, deleted,
                written_by, created_at
           FROM design_elements
          WHERE project_id=? AND element_id=? AND (? IS NULL OR revision=?)
          ORDER BY revision DESC LIMIT 1",
    )
    .bind(project.as_str())
    .bind(id)
    .bind(revision)
    .bind(revision)
    .fetch_optional(conn)
    .await?;
    row.map(|row| row.view(project)).transpose()
}

async fn latest_elements(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    include_deleted: bool,
) -> Result<Vec<DesignElement>, DesignError> {
    let rows: Vec<ElementRow> = sqlx::query_as(
        "SELECT element_id, revision, part_id, outcomes_json, content_json, deleted,
                written_by, created_at
           FROM design_elements AS element
          WHERE project_id=? AND (? OR deleted=0)
            AND revision=(
                SELECT MAX(newer.revision) FROM design_elements AS newer
                 WHERE newer.project_id=element.project_id AND newer.element_id=element.element_id
            )
          ORDER BY CAST(SUBSTR(element_id, 2) AS INTEGER), element_id",
    )
    .bind(project.as_str())
    .bind(include_deleted)
    .fetch_all(conn)
    .await?;
    rows.into_iter().map(|row| row.view(project)).collect()
}

async fn recorded(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    at: &str,
) -> Result<DesignElement, DesignError> {
    let (id, revision) = at
        .rsplit_once('@')
        .and_then(|(id, number)| Some((id, number.parse::<i64>().ok()?)))
        .ok_or(StorageError::NotFound("element command outcome"))?;
    find_revision(conn, project, id, Some(revision))
        .await?
        .ok_or_else(|| StorageError::NotFound("element revision").into())
}

async fn editable(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    id: &str,
    expected: i64,
) -> Result<DesignElement, DesignError> {
    let element = find_revision(conn, project, id, None)
        .await?
        .ok_or_else(|| invalid(format!("{id} is not an element of this project")))?;
    let latest = element.revision;
    if latest != expected {
        return Err(DesignError::ElementStale(format!(
            "{id} is at revision {latest}, not {expected}"
        )));
    }
    if element.deleted {
        return Err(invalid(format!("{id} is deleted; write a new element")));
    }
    Ok(element)
}

async fn next_id(conn: &mut SqliteConnection, project: &ProjectId) -> Result<String, DesignError> {
    let highest: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(CAST(SUBSTR(element_id, 2) AS INTEGER)), 0)
           FROM design_elements WHERE project_id=?",
    )
    .bind(project.as_str())
    .fetch_one(conn)
    .await?;
    Ok(format!("E{}", highest + 1))
}

async fn put_element_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    input: ElementInput,
    thread: &str,
    ts: &str,
) -> Result<DesignElement, DesignError> {
    let input = ElementRules::normalise(input)?;
    let kind = input.content.kind();
    let (id, revision) = if let (Some(id), Some(expected)) = (input.id, input.expected_revision) {
        let current = editable(conn, project, &id, expected).await?;
        let current_kind = current.content.kind();
        if current_kind != kind {
            return Err(invalid(format!(
                "{id} is a {}; its kind does not change",
                current_kind.as_str()
            )));
        }
        (id, current.revision + 1)
    } else {
        (next_id(conn, project).await?, 1)
    };
    check_placement(conn, project, &input.part_id, &input.outcomes, &kind).await?;

    let live = latest_elements(conn, project, false).await?;
    check_identity(&id, &input.content, &live)?;
    match &input.content {
        ElementContent::Table(table) => check_columns(&id, table, &live)?,
        ElementContent::Screen(screen) => check_links(&id, screen, &live)?,
    }
    if revision == 1 && live.len() >= LIVE_LIMIT {
        return Err(invalid(format!(
            "this project has {LIVE_LIMIT} tables and screens; delete one first"
        )));
    }
    Ok(DesignElement {
        id,
        revision,
        project_id: project.clone(),
        part_id: input.part_id,
        outcomes: input.outcomes,
        content: input.content,
        deleted: false,
        written_by: thread.into(),
        created_at: ts.into(),
    })
}

async fn check_placement(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    part: &PartId,
    outcomes: &[crate::OutcomeId],
    kind: &ElementKind,
) -> Result<(), DesignError> {
    let actual = Design::part_in(conn, project, part)
        .await
        .map_err(|error| match error {
            StorageError::NotFound(_) => {
                invalid(format!("part {part} is not a part of this project"))
            }
            other => other.into(),
        })?;
    let required = match kind {
        ElementKind::Table => "database",
        ElementKind::Screen => "frontend",
    };
    let actual_kind = actual.content.kind.as_deref().map(str::trim);
    if actual_kind != Some(required) {
        let actual_kind = actual_kind
            .filter(|kind| !kind.is_empty())
            .unwrap_or("of no kind");
        return Err(invalid(format!(
            "a {} belongs to a part of kind {required}; {part} is {actual_kind}",
            kind.as_str()
        )));
    }
    for outcome in outcomes {
        OutcomesRows::load(conn, project, outcome)
            .await
            .map_err(|error| match error {
                StorageError::NotFound(_) => invalid(format!(
                    "outcome {outcome} is not in this project's roadmap"
                )),
                other => other.into(),
            })?;
    }
    Ok(())
}

fn check_identity(
    id: &str,
    content: &ElementContent,
    live: &[DesignElement],
) -> Result<(), DesignError> {
    let kind = content.kind();
    let name = content.name();
    let folded = name.to_lowercase();
    for other in live.iter().filter(|other| other.id != id) {
        if other.content.kind() == kind && other.content.name().to_lowercase() == folded {
            return Err(invalid(format!(
                "a {} named `{name}` already exists: {}",
                kind.as_str(),
                other.id
            )));
        }
        if let (ElementContent::Screen(screen), ElementContent::Screen(previous)) =
            (content, &other.content)
            && let Some(route) = &screen.route
            && previous.route.as_ref() == Some(route)
        {
            return Err(invalid(format!(
                "the route `{route}` is already {}'s",
                other.id
            )));
        }
    }
    Ok(())
}

fn check_columns(
    id: &str,
    table: &TableContent,
    live: &[DesignElement],
) -> Result<(), DesignError> {
    let names: HashSet<_> = table
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    for column in &table.columns {
        let Some(target) = &column.references else {
            continue;
        };
        let has_column = if target.table == id {
            names.contains(target.column.as_str())
        } else {
            let Some(ElementContent::Table(target_table)) = live
                .iter()
                .find(|other| other.id == target.table)
                .map(|other| &other.content)
            else {
                return Err(invalid(format!(
                    "`{}` references {}, which is not a table of this project",
                    column.name, target.table
                )));
            };
            target_table
                .columns
                .iter()
                .any(|col| col.name == target.column)
        };
        if !has_column {
            return Err(invalid(format!(
                "`{}` references {}.{}, which {} does not have",
                column.name, target.table, target.column, target.table
            )));
        }
    }
    for other in live.iter().filter(|other| other.id != id) {
        if let ElementContent::Table(other_table) = &other.content {
            for column in &other_table.columns {
                if let Some(target) = &column.references
                    && target.table == id
                    && !names.contains(target.column.as_str())
                {
                    return Err(invalid(format!(
                        "{}.{} is referenced by {}.{} ({}); change {} first",
                        table.name,
                        target.column,
                        other_table.name,
                        column.name,
                        other.id,
                        other.id
                    )));
                }
            }
        }
    }
    Ok(())
}

fn check_links(
    id: &str,
    screen: &ScreenContent,
    live: &[DesignElement],
) -> Result<(), DesignError> {
    for link in &screen.leads_to {
        if link.screen != id
            && !live.iter().any(|other| {
                other.id == link.screen && matches!(other.content, ElementContent::Screen(_))
            })
        {
            return Err(invalid(format!(
                "{} is not a screen of this project",
                link.screen
            )));
        }
    }
    Ok(())
}

async fn delete_element_in(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    input: ElementDelete,
    thread: &str,
    ts: &str,
) -> Result<DesignElement, DesignError> {
    let id = input.id.trim();
    let current = editable(conn, project, id, input.expected_revision).await?;
    for other in latest_elements(conn, project, false).await? {
        if other.id == id {
            continue;
        }
        let detail = match &other.content {
            ElementContent::Table(table) => table
                .columns
                .iter()
                .find(|column| {
                    column
                        .references
                        .as_ref()
                        .is_some_and(|target| target.table == id)
                })
                .map(|column| format!("{}.{}", table.name, column.name)),
            ElementContent::Screen(screen) => screen
                .leads_to
                .iter()
                .any(|link| link.screen == id)
                .then(|| "leads to it".to_owned()),
        };
        if let Some(detail) = detail {
            return Err(invalid(format!(
                "{id} is referenced by {} ({detail}); change {} first",
                other.id, other.id
            )));
        }
    }
    Ok(DesignElement {
        revision: current.revision + 1,
        deleted: true,
        written_by: thread.into(),
        created_at: ts.into(),
        ..current
    })
}

async fn insert(conn: &mut SqliteConnection, element: &DesignElement) -> Result<(), DesignError> {
    sqlx::query(
        "INSERT INTO design_elements (
            project_id, element_id, revision, kind, name, part_id, outcomes_json, content_json,
            deleted, written_by, created_at
         ) VALUES (?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(element.project_id.as_str())
    .bind(&element.id)
    .bind(element.revision)
    .bind(element.content.kind().as_str())
    .bind(element.content.name())
    .bind(element.part_id.as_str())
    .bind(serde_json::to_string(&element.outcomes)?)
    .bind(serde_json::to_string(&element.content)?)
    .bind(element.deleted)
    .bind(&element.written_by)
    .bind(&element.created_at)
    .execute(conn)
    .await?;
    Ok(())
}

fn invalid(message: String) -> DesignError {
    DesignError::InvalidElement(message)
}

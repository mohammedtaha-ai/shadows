//! Live one-way project reach inside a caller's database snapshot.

use sqlx::SqliteConnection;

use crate::code::Code;
use crate::projects::{ProjectId, Projects};
use core_db::StorageError;

impl Code {
    /// Live same-project or direct-link reach in the caller's snapshot.
    pub(crate) async fn project_reachable_in(
        conn: &mut SqliteConnection,
        source: &ProjectId,
        target: &ProjectId,
    ) -> Result<bool, StorageError> {
        for project in [source, target] {
            match Projects::require_live_in(conn, project).await {
                Ok(()) => {}
                Err(StorageError::NotFound(_)) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        if source == target {
            return Ok(true);
        }
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM project_link
              WHERE project_id = ? AND linked_project_id = ?)",
        )
        .bind(source.as_str())
        .bind(target.as_str())
        .fetch_one(&mut *conn)
        .await?)
    }

    /// Resolves a live project within the origin's one-way scope in its snapshot.
    pub(crate) async fn selected_project_in(
        conn: &mut SqliteConnection,
        origin: &ProjectId,
        slug: Option<&str>,
    ) -> Result<ProjectId, crate::CoreError> {
        let selected = match slug {
            None => origin.clone(),
            Some(slug) => {
                Projects::by_slug_in(conn, slug)
                    .await?
                    .ok_or(crate::CoreError::Refused {
                        code: crate::ErrorCode::GrantScope,
                        message: "outside what the grant allows".into(),
                    })?
                    .id
            }
        };
        if !Self::project_reachable_in(conn, origin, &selected).await? {
            return Err(crate::CoreError::Refused {
                code: crate::ErrorCode::GrantScope,
                message: "outside what the grant allows".into(),
            });
        }
        Ok(selected)
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::Code;

    #[tokio::test]
    async fn link_reads_keep_the_callers_snapshot_and_replay_removed_projects() {
        let db = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let core = crate::testing::app(db.path().join("code.sqlite3"))
            .await
            .unwrap();
        let home = core
            .project_lifecycle()
            .create("home".into(), "home", "Home", dir.path().to_str().unwrap())
            .await
            .unwrap();
        let linked = core
            .project_lifecycle()
            .create(
                "linked".into(),
                "linked",
                "Linked",
                dir.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let code = core.code();
        let first = core
            .plans()
            .link_projects("link".into(), &home.id, &linked.id)
            .await
            .unwrap();
        let storage = &code.inner.store.storage;
        let mut snapshot = storage.reader().begin().await.unwrap();
        assert!(
            Code::project_reachable_in(&mut snapshot, &home.id, &linked.id)
                .await
                .unwrap()
        );
        core.project_lifecycle()
            .remove("remove".into(), &linked.id)
            .await
            .unwrap();
        assert_eq!(
            Code::selected_project_in(&mut snapshot, &home.id, Some("linked"))
                .await
                .unwrap(),
            linked.id,
            "project metadata and links must use the same snapshot"
        );
        snapshot.commit().await.unwrap();
        let mut snapshot = storage.reader().begin().await.unwrap();
        assert!(
            !Code::project_reachable_in(&mut snapshot, &home.id, &linked.id)
                .await
                .unwrap()
        );
        assert!(matches!(
            Code::selected_project_in(&mut snapshot, &home.id, Some("linked")).await,
            Err(crate::CoreError::Refused {
                code: crate::ErrorCode::GrantScope,
                ..
            })
        ));
        snapshot.commit().await.unwrap();
        let replay = core
            .plans()
            .link_projects("link".into(), &home.id, &linked.id)
            .await
            .unwrap();
        assert_eq!(replay.project, first.project);
        assert_eq!(replay.linked, first.linked);
        assert_eq!(replay.created_at, first.created_at);
        assert!(code.links(&home.id).await.unwrap().is_empty());
        core.code().shut_down().await;
    }
}

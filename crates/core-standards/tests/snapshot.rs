use core_db::Storage;
use core_db::StorageError;
use core_projects::Projects;
use core_standards::{Standards, StandardsError};
use std::sync::Arc;

#[tokio::test]
async fn additions_reads_keep_the_snapshot_and_scope_versions_to_the_project() {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&db.path().join("standards.sqlite3"))
            .await
            .unwrap(),
    );
    let projects = Projects::new(storage.clone());
    let project = projects
        .create(
            "project".into(),
            "project",
            "Project",
            dir.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let other = projects
        .create(
            "other".into(),
            "other",
            "Other",
            dir.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let standards = Standards::new(storage.clone());
    let first = standards
        .save_standards_additions("first".into(), &project.id, Default::default())
        .await
        .unwrap();
    let mut snapshot = storage.reader().begin().await.unwrap();
    assert_eq!(
        Standards::current_in(&mut snapshot, &project.id)
            .await
            .unwrap(),
        Some(first.clone())
    );
    standards
        .save_standards_additions("second".into(), &project.id, Default::default())
        .await
        .unwrap();
    let removed = project.id.clone();
    storage
        .write_txn(move |conn| {
            Box::pin(
                async move { Projects::mark_removed_in(conn, &removed, &Storage::now()).await },
            )
        })
        .await
        .unwrap();
    assert_eq!(
        Standards::current_in(&mut snapshot, &project.id)
            .await
            .unwrap(),
        Some(first.clone()),
        "liveness and additions must use the same snapshot"
    );
    assert_eq!(
        Standards::load_version_in(&mut snapshot, &project.id, &first.id)
            .await
            .unwrap(),
        first
    );
    assert!(matches!(
        Standards::load_version_in(&mut snapshot, &other.id, &first.id).await,
        Err(StorageError::NotFound("standards additions"))
    ));
    assert!(matches!(
        Standards::load_version_in(&mut snapshot, &other.id, "missing").await,
        Err(StorageError::NotFound("standards additions"))
    ));
    snapshot.commit().await.unwrap();
    assert!(matches!(
        standards.current(&project.id).await,
        Err(StandardsError::Storage(StorageError::NotFound("project")))
    ));
    assert_eq!(
        standards
            .save_standards_additions("first".into(), &project.id, Default::default())
            .await
            .unwrap(),
        first,
        "removal must not change an immutable replay result"
    );

    storage.close().await.unwrap();
}

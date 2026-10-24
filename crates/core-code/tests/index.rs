use std::sync::Arc;

use core_code::{Asker, Code, CodeConfig, CodeError};
use core_db::Storage;
use core_projects::Projects;

#[tokio::test]
async fn startup_order_uses_creation_sequence_then_project_id_for_ties() {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&db.path().join("code.sqlite3"))
            .await
            .unwrap(),
    );
    let projects = Projects::new(storage.clone());
    let mut ids = Vec::new();
    for slug in ["zulu", "alpha", "middle"] {
        ids.push(
            projects
                .create(slug.into(), slug, slug, dir.path().to_str().unwrap())
                .await
                .unwrap()
                .id,
        );
    }
    let code = Code::new(storage.clone(), projects.clone());
    code.start(CodeConfig::default()).await.unwrap();
    assert_eq!(
        code.active_for_test().await,
        ids.iter().rev().cloned().collect::<Vec<_>>(),
        "creation sequence must decide startup order"
    );
    code.shut_down().await;
    storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query("DELETE FROM durable_event WHERE kind = 'ProjectCreated'")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let code = Code::new(storage, projects);
    code.start(CodeConfig::default()).await.unwrap();
    assert_eq!(
        code.active_for_test().await,
        ids,
        "equal activity sequences must sort by project id"
    );
    code.shut_down().await;
}

#[tokio::test]
async fn indexing_uses_the_public_constructor_and_keeps_scope_refusals_distinct() {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("lib.rs"),
        "pub fn answer() {}
fn caller() { answer(); }
",
    )
    .unwrap();
    let storage = Arc::new(
        Storage::open(&db.path().join("code.sqlite3"))
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
    let code = Code::new(storage, projects);
    code.scan_for_test(&project.id).await.unwrap();
    let definitions = code
        .definitions(Asker::Person(&project.id), None, "answer")
        .await
        .unwrap();
    assert_eq!(definitions.hits.len(), 1);
    assert_eq!(definitions.hits[0].path, "lib.rs");
    let references = code
        .references(Asker::Person(&project.id), None, "answer")
        .await
        .unwrap();
    assert!(!references.hits.is_empty());
    assert!(
        references
            .hits
            .iter()
            .all(|hit| hit.matched_by.as_deref() == Some("name"))
    );
    assert!(matches!(
        code.definitions(Asker::Grant(&project.id), Some("outside"), "answer")
            .await,
        Err(CodeError::GrantScope(_))
    ));
    assert!(matches!(
        code.definitions(Asker::Person(&project.id), Some("outside"), "answer")
            .await,
        Err(CodeError::InvalidCommand(_))
    ));
    assert!(matches!(
        code.outline(Asker::Person(&project.id), None, "../outside")
            .await,
        Err(CodeError::InvalidCommand(_))
    ));
    assert!(matches!(
        code.set_active_limit("invalid".into(), 0).await,
        Err(CodeError::InvalidCommand(_))
    ));
    assert_eq!(code.settings().await.unwrap().active_limit, 5);
    code.shut_down().await;
}

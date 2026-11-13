use std::sync::Arc;

use codebase::{Asker, Codebase, CodebaseError};
use core_db::Storage;
use core_projects::Projects;

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
    let code = Codebase::new(storage.clone(), projects);
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
        Err(CodebaseError::GrantScope(_))
    ));
    assert!(matches!(
        code.definitions(Asker::Person(&project.id), Some("outside"), "answer")
            .await,
        Err(CodebaseError::InvalidCommand(_))
    ));
    assert!(matches!(
        code.outline(Asker::Person(&project.id), None, "../outside")
            .await,
        Err(CodebaseError::InvalidCommand(_))
    ));
    code.shut_down().await;

    storage.close().await.unwrap();
}

use std::sync::Arc;

use core_db::Storage;
use core_projects::Projects;

use super::Code;

#[tokio::test]
async fn hits_are_reloaded_when_indexing_finishes_before_the_stamp_check() {
    let db = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&db.path().join("code.sqlite3"))
            .await
            .unwrap(),
    );
    let projects = Projects::new(storage.clone());
    let project = projects
        .create("demo".into(), "demo", "Demo", dir.path().to_str().unwrap())
        .await
        .unwrap()
        .id;
    let code = Code::new(storage.clone(), projects);
    let file = dir.path().join("a.rs");
    std::fs::write(&file, "fn one() {}\n").unwrap();
    code.scan_for_test(&project).await.unwrap();
    let scope = code.inner.store.code_scope(&project).await.unwrap();
    let hits = code
        .inner
        .store
        .code_by_name(&scope, "one", "definition")
        .await
        .unwrap();
    assert_eq!(hits[0].line, 1);

    // A worker can commit the save after the hit query but before its stamp query.
    let tmp = dir.path().join(".a.rs.tmp");
    std::fs::write(&tmp, "\nfn one() {}\n").unwrap();
    std::fs::rename(tmp, file).unwrap();
    code.scan_for_test(&project).await.unwrap();
    assert!(code.stale(&project, dir.path(), ["a.rs"]).await.is_empty());
    assert!(
        code.recheck(&scope, &hits).await,
        "an up-to-date stamp must not let a question retain its older hits"
    );
    let refreshed = code
        .inner
        .store
        .code_by_name(&scope, "one", "definition")
        .await
        .unwrap();
    assert_eq!(refreshed[0].line, 2);

    storage.close().await.unwrap();
}

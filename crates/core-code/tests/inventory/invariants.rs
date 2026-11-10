use std::sync::Arc;

use core_db::Storage;
use core_projects::Projects;

use super::super::model::{InventoryPage, InventoryQuery, InventoryState};
use super::super::store::InventoryRow;
use super::super::{Asker, Code};

fn file(path: &str) -> InventoryRow {
    InventoryRow {
        path_key: path.to_string(),
        path: path.to_string(),
        kind: "file",
        excluded_reason: None,
    }
}

fn names(page: &InventoryPage) -> Vec<String> {
    page.entries.iter().map(|e| e.path.clone()).collect()
}

#[tokio::test]
async fn an_incomplete_listing_deletes_nothing() {
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
    let store = &code.inner.store;
    let me = Asker::Person(&project);
    let now = Storage::now();
    store
        .write_inventory(&project, vec![file("a.txt"), file("b.txt")], false, &now)
        .await
        .unwrap();
    store
        .write_inventory(&project, vec![file("a.txt")], true, &now)
        .await
        .unwrap();
    let page = code.inventory(me, InventoryQuery::default()).await.unwrap();
    assert_eq!(names(&page), ["a.txt", "b.txt"]);
    assert_eq!(page.state, InventoryState::Incomplete);
    assert_eq!(page.generation, 1);
    // A complete listing without b.txt removes it.
    store
        .write_inventory(&project, vec![file("a.txt")], false, &now)
        .await
        .unwrap();
    let after = code.inventory(me, InventoryQuery::default()).await.unwrap();
    assert_eq!(
        (names(&after), after.state, after.generation),
        (vec!["a.txt".to_string()], InventoryState::Ready, 2)
    );
    storage.close().await.unwrap();
}

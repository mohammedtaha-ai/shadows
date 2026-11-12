use std::path::Path;
use std::sync::Arc;

use core_code::{
    Asker, Code, CodeError, InventoryEntry, InventoryKind, InventoryPage, InventoryQuery,
    InventoryState,
};
use core_db::Storage;
use core_projects::{ProjectId, Projects};

/// A project on `dir` with its own database in a temporary folder, returned last.
async fn project_on(dir: &Path) -> (Arc<Storage>, Code, ProjectId, tempfile::TempDir) {
    let db = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&db.path().join("code.sqlite3"))
            .await
            .unwrap(),
    );
    let projects = Projects::new(storage.clone());
    let project = projects
        .create("demo".into(), "demo", "Demo", dir.to_str().unwrap())
        .await
        .unwrap()
        .id;
    let code = Code::new(storage.clone(), projects);
    (storage, code, project, db)
}

fn write(dir: &Path, path: &str, text: &str) {
    let full = dir.join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, text).unwrap();
}

fn names(p: &InventoryPage) -> Vec<String> {
    p.entries.iter().map(|e| e.path.clone()).collect()
}

#[tokio::test]
async fn inventory_lists_every_file_whatever_its_language() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    write(d, "composer.json", "{}");
    write(d, "app/Models/User.php", "<?php class User {}");
    write(d, "src/lib.rs", "pub fn a() {}");
    write(d, ".env", "APP_KEY=SECRET_CANARY");
    write(d, ".gitignore", "/vendor\n");
    write(d, "vendor/autoload.php", "<?php");
    write(d, "node_modules/x/index.js", "");
    write(d, ".git/config", "");
    let (storage, code, project, _db) = project_on(d).await;
    let me = Asker::Person(&project);

    let page = code.refresh_inventory(me).await.unwrap();
    assert_eq!(page.state, InventoryState::Ready);
    assert_eq!(page.generation, 1);
    assert!(page.next.is_none() && page.observed_at.is_some());
    let entry = |path: &str| page.entries.iter().find(|e| e.path == path).cloned();
    let file = |language: Option<&str>, has_symbols| {
        (
            InventoryKind::File,
            language.map(String::from),
            None::<String>,
            has_symbols,
        )
    };
    let shape = |e: InventoryEntry| (e.kind, e.language, e.excluded_reason, e.has_symbols);
    assert_eq!(shape(entry("composer.json").unwrap()), file(None, false));
    assert_eq!(
        shape(entry("app/Models/User.php").unwrap()),
        file(None, false)
    );
    assert_eq!(
        shape(entry("src/lib.rs").unwrap()),
        file(Some("rust"), true)
    );
    assert_eq!(shape(entry(".gitignore").unwrap()), file(None, false));
    assert_eq!(entry("app").unwrap().kind, InventoryKind::Directory);
    let excluded = |path: &str| {
        let e = entry(path).unwrap();
        assert_eq!(e.kind, InventoryKind::Excluded, "{path}");
        e.excluded_reason.unwrap()
    };
    assert_eq!(excluded(".env"), "secret");
    assert_eq!(excluded("vendor"), "ignored");
    assert_eq!(excluded("node_modules"), "dependencies");
    let paths: Vec<&str> = page.entries.iter().map(|e| e.path.as_str()).collect();
    assert!(
        paths
            .iter()
            .all(|p| !p.starts_with("vendor/") && !p.starts_with("node_modules/"))
    );
    assert!(
        paths
            .iter()
            .all(|p| *p != ".git" && !p.starts_with(".git/"))
    );
    let mut sorted = paths.clone();
    sorted.sort_by_key(|p| p.to_lowercase());
    assert_eq!(paths, sorted);
    assert!(
        !serde_json::to_string(&page)
            .unwrap()
            .contains("SECRET_CANARY")
    );
    storage.close().await.unwrap();
}

#[tokio::test]
async fn inventory_pages_in_path_order_and_refuses_a_stale_cursor() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.txt", "b.txt", "c.txt", "d.txt", "e.txt"] {
        write(dir.path(), name, "");
    }
    let (storage, code, project, _db) = project_on(dir.path()).await;
    let me = Asker::Person(&project);
    code.refresh_inventory(me).await.unwrap();

    let ask = |after: Option<String>, limit| InventoryQuery {
        prefix: String::new(),
        after,
        limit,
    };
    let one = code.inventory(me, ask(None, 2)).await.unwrap();
    assert_eq!(names(&one), ["a.txt", "b.txt"]);
    let two = code.inventory(me, ask(one.next.clone(), 2)).await.unwrap();
    assert_eq!(names(&two), ["c.txt", "d.txt"]);
    let three = code.inventory(me, ask(two.next.clone(), 2)).await.unwrap();
    assert_eq!(
        (names(&three), three.next),
        (vec!["e.txt".to_string()], None)
    );
    // Nothing changed: the generation stays, and an old cursor still reads.
    assert_eq!(code.refresh_inventory(me).await.unwrap().generation, 1);
    assert!(code.inventory(me, ask(one.next.clone(), 2)).await.is_ok());
    write(dir.path(), "f.txt", "");
    assert_eq!(code.refresh_inventory(me).await.unwrap().generation, 2);
    let stale = code.inventory(me, ask(one.next.clone(), 2)).await;
    assert!(matches!(stale, Err(CodeError::InventoryStale)), "{stale:?}");
    for (after, limit) in [(None, 0), (None, 201), (Some("garbage".to_string()), 2)] {
        let refused = code.inventory(me, ask(after, limit)).await;
        assert!(
            matches!(refused, Err(CodeError::InvalidCommand(_))),
            "{refused:?}"
        );
    }
    storage.close().await.unwrap();
}

#[tokio::test]
async fn inventory_prefix_lists_one_folder() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "app/a.php", "");
    write(dir.path(), "app/models/b.php", "");
    write(dir.path(), "app.php", "");
    let (storage, code, project, _db) = project_on(dir.path()).await;
    let me = Asker::Person(&project);
    code.refresh_inventory(me).await.unwrap();

    let query = InventoryQuery {
        prefix: "app".into(),
        ..Default::default()
    };
    let page = code.inventory(me, query).await.unwrap();
    assert_eq!(
        names(&page),
        ["app", "app/a.php", "app/models", "app/models/b.php"]
    );
    let query = InventoryQuery {
        prefix: "../x".into(),
        ..Default::default()
    };
    let out = code.inventory(me, query).await;
    assert!(matches!(out, Err(CodeError::InvalidCommand(_))));
    storage.close().await.unwrap();
}

#[tokio::test]
async fn inventory_is_indexing_before_its_first_listing_and_missing_without_a_folder() {
    let dir = tempfile::tempdir().unwrap();
    let (storage, code, project, _db) = project_on(dir.path()).await;
    let me = Asker::Person(&project);

    let before = code.inventory(me, InventoryQuery::default()).await.unwrap();
    assert_eq!(
        (before.state, before.generation),
        (InventoryState::Indexing, 0)
    );
    assert!(before.entries.is_empty() && before.observed_at.is_none());
    std::fs::remove_dir_all(dir.path()).unwrap();
    let gone = code.refresh_inventory(me).await.unwrap();
    assert_eq!(gone.state, InventoryState::Missing);
    storage.close().await.unwrap();
}

#[tokio::test]
async fn removing_a_file_removes_its_entry_and_moves_the_generation() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.txt", "");
    write(dir.path(), "b.txt", "");
    let (storage, code, project, _db) = project_on(dir.path()).await;
    let me = Asker::Person(&project);
    code.refresh_inventory(me).await.unwrap();
    std::fs::remove_file(dir.path().join("b.txt")).unwrap();
    let page = code.refresh_inventory(me).await.unwrap();
    assert_eq!(names(&page), ["a.txt"]);
    assert_eq!(page.generation, 2);
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_link_out_of_the_folder_is_listed_and_never_entered() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "secret.txt", "outside");
    let link = dir.path().join("out");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .status()
            .unwrap();
        assert!(status.success());
    }
    let (storage, code, project, _db) = project_on(dir.path()).await;
    let page = code
        .refresh_inventory(Asker::Person(&project))
        .await
        .unwrap();

    let out = page.entries.iter().find(|e| e.path == "out").unwrap();
    assert_eq!(out.kind, InventoryKind::Excluded);
    assert_eq!(out.excluded_reason.as_deref(), Some("link"));
    assert!(page.entries.iter().all(|e| !e.path.starts_with("out/")));
    storage.close().await.unwrap();
}

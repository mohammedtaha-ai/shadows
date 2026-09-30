//! Spec §15.4–§15.5: the code index follows the files of a project's folder,
//! and the three questions answer from it.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::claude::ClaudeAdapter;
use shadows_core::testing::{
    CommandContext, ProjectDirectory, Runtime, Sessions, Storage, acp, fingerprint,
};
use shadows_core::{AppCore, Asker, CodeConfig, CoreParts, IndexState, ProjectId};

/// A core over a temporary database in `db`, with one project whose folder is `dir`.
async fn core_with_project() -> (
    Arc<AppCore>,
    ProjectId,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    let db = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&db.path().join("s.sqlite3")).await.unwrap());
    let dir = tempfile::tempdir().unwrap();
    let project = new_project(&storage, "demo", dir.path()).await;
    (core_over(storage).await, project, dir, db)
}

/// `n` projects, `p0` to `p<n-1>` created in that order and with no turn,
/// each on its own folder holding `src/p<i>.rs`: `fn p<i>() {}`. The folders
/// and the database live as long as the answer.
async fn core_with_projects(n: usize) -> (Arc<AppCore>, Vec<ProjectId>, Vec<tempfile::TempDir>) {
    let db = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&db.path().join("s.sqlite3")).await.unwrap());
    let (mut projects, mut dirs) = (Vec::new(), vec![db]);
    for i in 0..n {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let file = dir.path().join(format!("src/p{i}.rs"));
        std::fs::write(file, format!("fn p{i}() {{}}\n")).unwrap();
        projects.push(new_project(&storage, &format!("p{i}"), dir.path()).await);
        dirs.push(dir);
    }
    (core_over(storage).await, projects, dirs)
}

/// Polls `check` every 50 ms for up to 10 s.
async fn eventually<F: Future<Output = bool>>(check: impl Fn() -> F) {
    for _ in 0..200 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("not true within 10 s");
}

async fn new_project(storage: &Storage, slug: &str, dir: &std::path::Path) -> ProjectId {
    let ctx = CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: format!("c-{slug}"),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("project.create", &serde_json::json!({})),
    };
    storage
        .create_project(
            &ctx,
            slug,
            slug,
            &ProjectDirectory::resolve(dir).unwrap(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap()
        .id
}

async fn core_over(storage: Arc<Storage>) -> Arc<AppCore> {
    let no_adapter = Arc::new(ClaudeAdapter {
        node: "none".into(),
        adapter: "none".into(),
        agent: "none".into(),
        adapter_version: "none".into(),
        agent_version: "none".into(),
    });
    let (runtime, _) = Runtime::start(storage.clone()).await.unwrap();
    AppCore::assemble(CoreParts {
        storage: storage.clone(),
        runtime: Arc::new(runtime),
        // No turn runs here, so the adapter is never started: no `fake-acp` build.
        sessions: Sessions::new(no_adapter, storage, acp::test_config()),
        handles: Default::default(),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
    })
}

#[tokio::test]
async fn indexing_follows_the_files() {
    let (core, project, dir, _db) = core_with_project().await; // fixture: temp db + project on `dir`
    let w = |p: &str, s: &str| {
        let p = dir.path().join(p);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    };
    w("src/a.rs", "pub fn alpha() {}\n");
    w("src/b.rs", "fn beta() { alpha(); }\n");
    w("target/debug/gen.rs", "fn hidden_in_target() {}\n"); // never walked
    w("sub/.gitignore", "gen.rs\n"); // no .git anywhere
    w("sub/gen.rs", "fn hidden_by_nested_ignore() {}\n");
    w("big.rs", &"// x\n".repeat(300_000)); // > 1 MB
    let me = shadows_core::Asker::Person(&project);

    core.code().scan_for_test(&project).await.unwrap();
    let a = core.code().definitions(me, None, "alpha").await.unwrap();
    assert_eq!((a.hits[0].path.as_str(), a.hits[0].line), ("src/a.rs", 1));
    assert_eq!(a.hits[0].signature.as_deref(), Some("pub fn alpha() {}"));
    let r = core.code().references(me, None, "alpha").await.unwrap();
    assert_eq!(r.hits[0].path, "src/b.rs");
    assert_eq!(r.hits[0].matched_by.as_deref(), Some("name"));
    for hidden in ["hidden_in_target", "hidden_by_nested_ignore"] {
        assert!(
            core.code()
                .definitions(me, None, hidden)
                .await
                .unwrap()
                .hits
                .is_empty(),
            "{hidden}"
        );
    }
    let s = core.code().status(&project).await.unwrap();
    assert!(
        s.skipped
            .iter()
            .any(|k| k.reason == "too_large" && k.count == 1)
    );

    // Change one, delete one, add one; the next scan follows each.
    w("src/a.rs", "\n\npub fn alpha() {}\n");
    std::fs::remove_file(dir.path().join("src/b.rs")).unwrap();
    w("src/c.ts", "export function gamma() {}\n");
    core.code().scan_for_test(&project).await.unwrap();
    assert_eq!(
        core.code()
            .definitions(me, None, "alpha")
            .await
            .unwrap()
            .hits[0]
            .line,
        3
    );
    assert!(
        core.code()
            .references(me, None, "alpha")
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(
        core.code()
            .definitions(me, None, "gamma")
            .await
            .unwrap()
            .hits
            .len(),
        1
    );
    let o = core.code().outline(me, None, "src").await.unwrap();
    assert_eq!(
        o.hits.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(),
        ["alpha", "gamma"]
    );
    let near = core.code().definitions(me, None, "amm").await.unwrap();
    assert!(near.hits.is_empty() && near.suggestions == ["gamma"]);
}

#[tokio::test]
async fn watching_and_the_periodic_scan_keep_the_index_true() {
    let (core, project, dir, _db) = core_with_project().await;
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/a.rs"), "fn one() {}\n").unwrap();
    let fast = CodeConfig {
        rescan_every: Duration::from_millis(400),
        debounce: Duration::from_millis(100),
    };
    core.code().start(fast).await.unwrap();
    core.code().touch(&project).await;
    let me = Asker::Person(&project);
    let code = core.code();
    let defined = |name: &'static str| async move {
        !code
            .definitions(me, None, name)
            .await
            .unwrap()
            .hits
            .is_empty()
    };
    eventually(|| defined("one")).await;

    // An editor's save: write a temporary file, then rename it over the original.
    let tmp = dir.path().join("src/.a.rs.tmp");
    std::fs::write(&tmp, "\nfn one() {}\n").unwrap();
    std::fs::rename(&tmp, dir.path().join("src/a.rs")).unwrap();
    // The re-check answers the new line at once, without waiting for the watcher.
    let a = core.code().definitions(me, None, "one").await.unwrap();
    assert_eq!(a.hits[0].line, 2);

    // A file the watcher never reports: the periodic scan finds it.
    core.code().pause_watcher_for_test(&project).await;
    std::fs::write(dir.path().join("src/b.rs"), "fn two() {}\n").unwrap();
    eventually(|| defined("two")).await;
    core.code().shut_down_for_test().await;
}

#[tokio::test]
async fn only_the_most_recent_projects_are_watched() {
    let (core, projects, _dirs) = core_with_projects(6).await;
    core.code().start(CodeConfig::default()).await.unwrap();
    let active = core.code().active_for_test().await;
    assert_eq!(active.len(), 5);
    let sixth = projects.iter().find(|p| !active.contains(p)).unwrap();
    // Asking the sixth brings it in: its answer says it is indexing, not an error.
    let a = core.code().status(sixth).await.unwrap();
    assert!(matches!(a.state, IndexState::Inactive));
    let q = core
        .code()
        .definitions(Asker::Person(sixth), None, "p0")
        .await
        .unwrap();
    assert!(matches!(
        q.status[0].state,
        IndexState::Indexing { .. } | IndexState::Ready
    ));
    let now = core.code().active_for_test().await;
    assert_eq!(now.len(), 5);
    assert!(now.contains(sixth));
    assert!(
        !now.contains(&active[active.len() - 1]),
        "the least recently used left"
    );
    core.code().shut_down_for_test().await;
}

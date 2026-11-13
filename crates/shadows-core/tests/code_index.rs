//! The code index follows the files of a project's folder,
//! and the three questions answer from it.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::claude::ClaudeAdapter;
use shadows_core::testing::{CommandContext, ProjectDirectory, Storage, acp};
use shadows_core::{AppCore, Asker, CodebaseConfig, CoreParts, IndexState, ProjectId};

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

async fn new_project(storage: &Arc<Storage>, slug: &str, dir: &std::path::Path) -> ProjectId {
    let ctx = CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: format!("c-{slug}"),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: CommandContext::fingerprint("project.create", &serde_json::json!({})),
    };
    shadows_core::testing::create_project(
        storage,
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
    let no_adapter = Arc::new(ClaudeAdapter::new(
        "none".into(),
        "none".into(),
        "none".into(),
        "none".into(),
        "none".into(),
    ));
    let (runtime, _) = shadows_core::AppCore::runtime_for_tests(storage.clone())
        .await
        .unwrap();
    AppCore::assemble(CoreParts {
        operations: shadows_core::AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime: Arc::new(runtime),
        // No turn runs here, so the adapter is never started: no `fake-acp` build.
        sessions: shadows_core::AppCore::harness_for_tests(no_adapter, storage, acp::test_config()),
        handles: Default::default(),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
    })
}

#[tokio::test]
async fn indexing_follows_the_files() {
    // fixture: temp db + project on `dir`
    let (core, project, dir, _db) = core_with_project().await;
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

    core.codebase().scan_for_test(&project).await.unwrap();
    let a = core
        .codebase()
        .definitions(me, None, "alpha")
        .await
        .unwrap();
    assert_eq!((a.hits[0].path.as_str(), a.hits[0].line), ("src/a.rs", 1));
    assert_eq!(a.hits[0].signature.as_deref(), Some("pub fn alpha() {}"));
    let r = core.codebase().references(me, None, "alpha").await.unwrap();
    assert_eq!(r.hits[0].path, "src/b.rs");
    assert_eq!(r.hits[0].matched_by.as_deref(), Some("name"));
    for hidden in ["hidden_in_target", "hidden_by_nested_ignore"] {
        assert!(
            core.codebase()
                .definitions(me, None, hidden)
                .await
                .unwrap()
                .hits
                .is_empty(),
            "{hidden}"
        );
    }
    let s = core.codebase().status(&project).await.unwrap();
    assert!(
        s.skipped
            .iter()
            .any(|k| k.reason == "too_large" && k.count == 1)
    );

    // Change one, delete one, add one; the next scan follows each.
    w("src/a.rs", "\n\npub fn alpha() {}\n");
    std::fs::remove_file(dir.path().join("src/b.rs")).unwrap();
    w("src/c.ts", "export function gamma() {}\n");
    core.codebase().scan_for_test(&project).await.unwrap();
    assert_eq!(
        core.codebase()
            .definitions(me, None, "alpha")
            .await
            .unwrap()
            .hits[0]
            .line,
        3
    );
    assert!(
        core.codebase()
            .references(me, None, "alpha")
            .await
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(
        core.codebase()
            .definitions(me, None, "gamma")
            .await
            .unwrap()
            .hits
            .len(),
        1
    );
    let o = core.codebase().outline(me, None, "src").await.unwrap();
    assert_eq!(
        o.hits.iter().map(|h| h.name.as_str()).collect::<Vec<_>>(),
        ["alpha", "gamma"]
    );
    let near = core.codebase().definitions(me, None, "amm").await.unwrap();
    assert!(near.hits.is_empty() && near.suggestions == ["gamma"]);

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn watching_and_the_periodic_scan_keep_the_index_true() {
    let (core, project, dir, _db) = core_with_project().await;
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/a.rs"), "fn one() {}\n").unwrap();
    let fast = CodebaseConfig {
        rescan_every: Duration::from_millis(400),
        debounce: Duration::from_millis(100),
        ..CodebaseConfig::default()
    };
    core.codebase().start(fast).await.unwrap();
    core.codebase().touch(&project).await;
    let me = Asker::Person(&project);
    let code = core.codebase();
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
    let a = core.codebase().definitions(me, None, "one").await.unwrap();
    assert_eq!(a.hits[0].line, 2);

    // A file the watcher never reports: the periodic scan finds it.
    core.codebase().pause_watcher_for_test(&project).await;
    std::fs::write(dir.path().join("src/b.rs"), "fn two() {}\n").unwrap();
    eventually(|| defined("two")).await;
    core.codebase().shut_down_for_test().await;

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

/// Sends one message in a new thread of `project`. No adapter runs here, so the
/// turn itself fails to open its session; the use is what counts.
async fn send_a_message(core: &AppCore, project: &ProjectId) {
    let thread = core
        .threads()
        .create("t-send".into(), project, "Chat", None, None)
        .await
        .unwrap();
    let _ = core
        .turns()
        .send(
            thread.id.clone(),
            shadows_core::SendTurn {
                command_id: "send-1".into(),
                prompt: "hello".into(),
                attachments: vec![],
                model: "default".into(),
                mode: "default".into(),
                effort: None,
                focus: None,
                plan: None,
                client_tab: None,
            },
        )
        .await;
}

async fn inactive(core: &AppCore, project: &ProjectId) -> bool {
    matches!(
        core.codebase().status(project).await.unwrap().state,
        IndexState::Inactive
    )
}

#[tokio::test]
async fn a_sent_message_activates_its_project() {
    let (core, project, _dir, _db) = core_with_project().await;
    core.codebase()
        .start(CodebaseConfig::default())
        .await
        .unwrap();
    assert!(inactive(&core, &project).await);
    send_a_message(&core, &project).await;
    assert!(!inactive(&core, &project).await);
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
async fn listing_threads_does_not_activate() {
    let (core, project, _dir, _db) = core_with_project().await;
    core.codebase()
        .start(CodebaseConfig::default())
        .await
        .unwrap();
    core.threads().list(&project).await.unwrap();
    assert!(inactive(&core, &project).await);
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
async fn an_idle_project_stops() {
    let (core, project, _dir, _db) = core_with_project().await;
    core.codebase().idle_for_test(Duration::from_secs(1));
    core.codebase()
        .start(CodebaseConfig::default())
        .await
        .unwrap();
    core.codebase().touch(&project).await;
    assert!(!inactive(&core, &project).await);
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(inactive(&core, &project).await);
    assert!(core.codebase().active_for_test().await.is_empty());
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
async fn a_touch_does_not_activate_links() {
    let (core, projects, _dirs) = core_with_projects(2).await;
    core.plans()
        .link_projects("link-1".into(), &projects[0], &projects[1])
        .await
        .unwrap();
    core.codebase()
        .start(CodebaseConfig::default())
        .await
        .unwrap();
    core.codebase().touch(&projects[0]).await;
    assert_eq!(
        core.codebase().active_for_test().await,
        [projects[0].clone()]
    );
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
async fn a_held_project_outlives_the_idle_time() {
    let (core, project, _dir, _db) = core_with_project().await;
    core.codebase().idle_for_test(Duration::from_secs(1));
    core.codebase()
        .start(CodebaseConfig::default())
        .await
        .unwrap();
    let hold = core.codebase().hold(&project).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(!inactive(&core, &project).await, "held past the idle time");
    drop(hold);
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!inactive(&core, &project).await, "the drop is a use");
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(inactive(&core, &project).await);
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
async fn the_name_index_is_off_by_default() {
    let db = tempfile::tempdir().unwrap();
    let core = shadows_core::testing::app(db.path().join("s.sqlite3"))
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("lib.rs"), "pub fn answer() {}\n").unwrap();
    let project = core
        .projects()
        .create("c-off".into(), "off", "Off", dir.path().to_str().unwrap())
        .await
        .unwrap()
        .id;
    core.codebase().scan_for_test(&project).await.unwrap();
    let status = core.codebase().status(&project).await.unwrap();
    assert_eq!(status.files, 1, "the file row is written");
    let found = core
        .codebase()
        .definitions(Asker::Person(&project), None, "answer")
        .await
        .unwrap();
    assert!(found.hits.is_empty(), "no tag is extracted");
    core.shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

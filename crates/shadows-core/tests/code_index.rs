//! Spec §15.4–§15.5: the code index follows the files of a project's folder,
//! and the three questions answer from it.

use std::sync::Arc;

use shadows_agent::claude::ClaudeAdapter;
use shadows_core::testing::{
    CommandContext, ProjectDirectory, Runtime, Sessions, Storage, acp, fingerprint,
};
use shadows_core::{AppCore, CoreParts, ProjectId};

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
    let ctx = CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c1".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint("project.create", &serde_json::json!({})),
    };
    let project = storage
        .create_project(
            &ctx,
            "demo",
            "Demo",
            &ProjectDirectory::resolve(dir.path()).unwrap(),
            &shadows_agent::policy::default_modes(),
        )
        .await
        .unwrap();
    let no_adapter = Arc::new(ClaudeAdapter {
        node: "none".into(),
        adapter: "none".into(),
        agent: "none".into(),
        adapter_version: "none".into(),
        agent_version: "none".into(),
    });
    let (runtime, _) = Runtime::start(storage.clone()).await.unwrap();
    let core = AppCore::assemble(CoreParts {
        storage: storage.clone(),
        runtime: Arc::new(runtime),
        // No turn runs here, so the adapter is never started: no `fake-acp` build.
        sessions: Sessions::new(no_adapter, storage, acp::test_config()),
        handles: Default::default(),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
    });
    (core, project.id, dir, db)
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

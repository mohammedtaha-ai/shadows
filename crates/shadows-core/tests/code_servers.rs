//! A project's language servers follow its files: the catalog, the rule "the files
//! choose", and each server's state.

use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use shadows_agent::claude::ClaudeAdapter;
use shadows_core::testing::{CommandContext, ProjectDirectory, Storage, acp};
use shadows_core::{
    AddedServer, AppCore, Asker, Codebase, CodebaseConfig, CoreParts, LspAnswer, LspOperation,
    LspQuestion, ProjectId,
};

const A_GO: &str = "package demo\n\nfunc Hello() string {\n\treturn \"hi\"\n}\n";
const A_GO_MOVED: &str =
    "package demo\n\n// moved\n// down\nfunc Hello() string {\n\treturn \"hi\"\n}\n";
const B_GO: &str = "package demo\n\nfunc Greet() string {\n\treturn Hello()\n}\n";

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
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "c-demo".into(),
        command_kind: "project.create".into(),
        command_schema_ver: 1,
        request_fingerprint: CommandContext::fingerprint("project.create", &serde_json::json!({})),
    };
    let project = shadows_core::testing::create_project(
        &storage,
        &ctx,
        "demo",
        "demo",
        &ProjectDirectory::resolve(dir.path()).unwrap(),
        &shadows_agent::policy::default_modes(),
    )
    .await
    .unwrap()
    .id;
    let no_adapter = Arc::new(ClaudeAdapter::new(
        "none".into(),
        "none".into(),
        "none".into(),
        "none".into(),
        "none".into(),
    ));
    let (runtime, _) = AppCore::runtime_for_tests(storage.clone()).await.unwrap();
    let core = AppCore::assemble(CoreParts {
        operations: AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime: Arc::new(runtime),
        sessions: AppCore::harness_for_tests(no_adapter, storage, acp::test_config()),
        handles: Default::default(),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: acp::MCP_URL.to_string(),
    });
    (core, project, dir, db)
}

/// Fast enough that a test waits seconds, not a minute.
fn fast() -> CodebaseConfig {
    CodebaseConfig {
        rescan_every: Duration::from_secs(2),
        debounce: Duration::from_millis(200),
        ..CodebaseConfig::default()
    }
}

async fn eventually<F: Future<Output = bool>>(seconds: u64, check: impl Fn() -> F) {
    for _ in 0..seconds * 10 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("not true within {seconds} s");
}

/// The state of server `id` in `project`, as its JSON tag (`running`, `idle`, ...).
async fn state(core: &AppCore, project: &ProjectId, id: &str) -> String {
    let all = core
        .codebase()
        .servers(Asker::Person(project))
        .await
        .unwrap();
    let server = all.iter().find(|s| s.id == id).expect("listed");
    serde_json::to_value(&server.state).unwrap()["state"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn running(core: &AppCore, project: &ProjectId) -> Vec<String> {
    let all = core
        .codebase()
        .servers(Asker::Person(project))
        .await
        .unwrap();
    all.into_iter()
        .filter(|s| serde_json::to_value(&s.state).unwrap()["state"] == "running")
        .map(|s| s.id)
        .collect()
}

async fn active(core: &AppCore, project: &ProjectId) {
    core.codebase().start(fast()).await.unwrap();
    core.codebase().touch(project).await;
}

fn write(dir: &Path, path: &str, text: &str) {
    std::fs::write(dir.join(path), text).unwrap();
}

#[test]
fn the_catalog_has_the_twelve_servers() {
    let catalog = Codebase::catalog_for_test();
    assert_eq!(catalog.len(), 12);
    let clangd = &catalog.iter().find(|(id, _)| id == "clangd").unwrap().1;
    assert!(clangd.contains(&".C".to_string()) && clangd.contains(&".c".to_string()));
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn a_go_file_wants_gopls() {
    let (core, project, dir, _db) = core_with_project().await;
    write(dir.path(), "go.mod", "module example.com/demo\n\ngo 1.21\n");
    write(dir.path(), "main.go", "package main\n\nfunc main() {}\n");
    active(&core, &project).await;
    eventually(60, || async {
        state(&core, &project, "gopls").await == "running"
    })
    .await;
    assert_eq!(running(&core, &project).await, ["gopls"]);
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn a_new_extension_starts_its_server() {
    let (core, project, dir, _db) = core_with_project().await;
    active(&core, &project).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    write(dir.path(), "go.mod", "module example.com/demo\n\ngo 1.21\n");
    write(dir.path(), "main.go", "package main\n\nfunc main() {}\n");
    eventually(60, || async {
        state(&core, &project, "gopls").await == "running"
    })
    .await;
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn a_watched_change_reaches_the_server() {
    let (core, project, dir, _db) = core_with_project().await;
    write(dir.path(), "go.mod", "module example.com/demo\n\ngo 1.21\n");
    write(dir.path(), "a.go", A_GO);
    write(dir.path(), "b.go", B_GO);
    active(&core, &project).await;
    eventually(60, || async {
        state(&core, &project, "gopls").await == "running"
    })
    .await;
    assert_eq!(hello_line(&core, &project).await, 3);
    write(dir.path(), "a.go", A_GO_MOVED);
    // The watcher reports the write; no question comes between.
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(hello_line(&core, &project).await, 5);
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn a_scanned_change_reaches_the_server() {
    let (core, project, dir, _db) = core_with_project().await;
    write(dir.path(), "go.mod", "module example.com/demo\n\ngo 1.21\n");
    write(dir.path(), "a.go", A_GO);
    write(dir.path(), "b.go", B_GO);
    // No periodic scan within the test: only the refresh below scans.
    let config = CodebaseConfig {
        debounce: Duration::from_millis(200),
        ..CodebaseConfig::default()
    };
    core.codebase().start(config).await.unwrap();
    core.codebase().touch(&project).await;
    eventually(60, || async {
        state(&core, &project, "gopls").await == "running"
    })
    .await;
    assert_eq!(hello_line(&core, &project).await, 3);
    // The watcher's report is dropped: only the scan can tell gopls.
    core.codebase().pause_watcher_for_test(&project).await;
    write(dir.path(), "a.go", A_GO_MOVED);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        hello_line(&core, &project).await,
        3,
        "gopls was told before the scan"
    );
    core.codebase()
        .refresh_inventory(Asker::Person(&project))
        .await
        .unwrap();
    assert_eq!(hello_line(&core, &project).await, 5);
    core.codebase().shut_down_for_test().await;
}

/// The one-based line `code_lsp` answers for `Hello`'s definition, asked from its
/// call in b.go.
async fn hello_line(core: &AppCore, project: &ProjectId) -> u64 {
    let question = LspQuestion {
        operation: LspOperation::GoToDefinition,
        file: Some("b.go".into()),
        line: Some(4),
        character: Some(10),
        query: None,
        project: None,
    };
    let answer = core
        .codebase()
        .lsp(Asker::Person(project), question)
        .await
        .unwrap();
    let LspAnswer::Answered { result, .. } = answer else {
        panic!("no server answered: {answer:?}");
    };
    let first = result
        .as_array()
        .map_or(result.clone(), |all| all[0].clone());
    assert_eq!(first["path"], "a.go", "{result}");
    first["line"].as_u64().unwrap()
}

/// Adds a server beside the catalog, as the person does.
async fn add(core: &AppCore, id: &str, command: &str, args: &[&str], extension: &str) {
    let server = AddedServer {
        id: id.into(),
        command: command.into(),
        args: args.iter().map(|a| a.to_string()).collect(),
        extensions: vec![extension.into()],
    };
    core.codebase()
        .add_server(format!("add-{id}"), server)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_missing_command_reads_not_installed() {
    let (core, project, dir, _db) = core_with_project().await;
    add(&core, "zz-server", "no-such-server-xyz", &[], ".zz").await;
    write(dir.path(), "x.zz", "zz\n");
    active(&core, &project).await;
    core.codebase().scan_for_test(&project).await.unwrap();
    assert_eq!(state(&core, &project, "zz-server").await, "not_installed");
    core.codebase().shut_down_for_test().await;
}

#[tokio::test]
async fn a_server_that_exits_reads_failed() {
    let (core, project, dir, _db) = core_with_project().await;
    let (command, args): (&str, &[&str]) = if cfg!(windows) {
        ("cmd", &["/c", "exit 3"])
    } else {
        ("sh", &["-c", "exit 3"])
    };
    add(&core, "quits", command, args, ".yy").await;
    write(dir.path(), "x.yy", "yy\n");
    active(&core, &project).await;
    eventually(30, || async {
        state(&core, &project, "quits").await == "failed"
    })
    .await;
    core.codebase().shut_down_for_test().await;
}

#[cfg(windows)]
#[test]
fn a_cmd_shim_on_path_is_found() {
    let bin = tempfile::tempdir().unwrap();
    let shim = bin.path().join("fake-ls.cmd");
    std::fs::write(&shim, "@echo off\r\n").unwrap();
    let path = std::env::join_paths([bin.path()]).unwrap();
    let found = Codebase::find_command("fake-ls", &path).expect("found");
    assert_eq!(
        found.to_string_lossy().to_lowercase(),
        shim.to_string_lossy().to_lowercase()
    );
}

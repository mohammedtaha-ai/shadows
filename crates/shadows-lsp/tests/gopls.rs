use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};
use shadows_lsp::{FileChange, LspError, Server, ServerSpec};

const WAIT: Duration = Duration::from_secs(60);
const A_GO: &str = "package demo\n\nfunc Hello() string {\n\treturn \"hi\"\n}\n";
const A_GO_MOVED: &str =
    "package demo\n\n// moved\n// down\nfunc Hello() string {\n\treturn \"hi\"\n}\n";
const B_GO: &str = "package demo\n\nfunc Greet() string {\n\treturn Hello()\n}\n";

fn module() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("go.mod"),
        "module example.com/demo\n\ngo 1.21\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("a.go"), A_GO).unwrap();
    std::fs::write(dir.path().join("b.go"), B_GO).unwrap();
    dir
}

fn root(dir: &tempfile::TempDir) -> PathBuf {
    dunce_free(dir.path().canonicalize().unwrap())
}

/// `canonicalize` on Windows answers `\\?\C:\...`, which no file URI carries.
fn dunce_free(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => path,
    }
}

async fn gopls(root: &Path) -> Server {
    Server::start(
        ServerSpec {
            executable: PathBuf::from("gopls"),
            args: vec![],
            root: root.to_path_buf(),
            env: std::env::vars().collect(),
        },
        WAIT,
    )
    .await
    .expect("gopls starts")
}

fn at(path: &Path, line: u32, character: u32) -> Value {
    json!({
        "textDocument": { "uri": url(path) },
        "position": { "line": line, "character": character },
    })
}

fn url(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    if text.starts_with('/') {
        format!("file://{text}")
    } else {
        format!("file:///{text}")
    }
}

async fn definition_line(server: &Server, b: &Path) -> (String, u64) {
    let answer = server
        .ask(
            b,
            "go",
            B_GO.into(),
            "textDocument/definition",
            at(b, 3, 9),
            WAIT,
        )
        .await
        .expect("a definition");
    let location = match &answer {
        Value::Array(all) => all[0].clone(),
        other => other.clone(),
    };
    (
        location["uri"].as_str().unwrap().to_owned(),
        location["range"]["start"]["line"].as_u64().unwrap(),
    )
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn gopls_answers_a_definition() {
    let dir = module();
    let root = root(&dir);
    let server = gopls(&root).await;
    let (uri, line) = definition_line(&server, &root.join("b.go")).await;
    assert!(uri.ends_with("a.go"), "{uri}");
    assert_eq!(line, 2);
    server.shutdown().await;
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn an_edit_on_disk_is_seen_by_the_next_question() {
    let dir = module();
    let root = root(&dir);
    let server = gopls(&root).await;
    let b = root.join("b.go");
    assert_eq!(definition_line(&server, &b).await.1, 2);
    std::fs::write(root.join("a.go"), A_GO_MOVED).unwrap();
    server
        .files_changed(&[(root.join("a.go"), FileChange::Changed)])
        .await
        .unwrap();
    assert_eq!(definition_line(&server, &b).await.1, 4);
    server.shutdown().await;
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn gopls_answers_references() {
    let dir = module();
    let root = root(&dir);
    let server = gopls(&root).await;
    let a = root.join("a.go");
    let mut params = at(&a, 2, 6);
    params["context"] = json!({ "includeDeclaration": true });
    let answer = server
        .ask(
            &a,
            "go",
            A_GO.into(),
            "textDocument/references",
            params,
            WAIT,
        )
        .await
        .unwrap();
    assert_eq!(answer.as_array().map(Vec::len), Some(2), "{answer}");
    server.shutdown().await;
}

#[tokio::test]
async fn a_missing_command_is_a_spawn_error() {
    let dir = tempfile::tempdir().unwrap();
    let started = Server::start(
        ServerSpec {
            executable: PathBuf::from("no-such-server-xyz"),
            args: vec![],
            root: root(&dir),
            env: std::env::vars().collect(),
        },
        WAIT,
    )
    .await;
    assert!(
        matches!(started, Err(LspError::Spawn(_))),
        "{:?}",
        started.err()
    );
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn shutdown_reaps_the_child() {
    let dir = module();
    let root = root(&dir);
    let server = gopls(&root).await;
    let pid = server.pid().expect("a pid");
    server.shutdown().await;
    assert!(!alive(pid), "pid {pid} still runs");
}

fn alive(pid: u32) -> bool {
    if cfg!(windows) {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    } else {
        Path::new(&format!("/proc/{pid}")).exists()
    }
}

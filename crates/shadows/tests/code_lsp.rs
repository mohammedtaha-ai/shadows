//! `code_lsp`: the project's language server asked over MCP, the file checked
//! inside the project first, and the three name-index tools gone.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::json;
#[allow(unused_imports)]
use shadows_core::testing::acp;
use shadows_core::{AddedServer, CodebaseConfig, GrantKind, Grants};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/plan.rs"]
mod plan;

use listening::{Listening, call, listening_app, ok, project_client, refused};

const A_GO: &str = "package demo\n\nfunc Hello() string {\n\treturn \"hi\"\n}\n";
const B_GO: &str = "package demo\n\nfunc Greet() string {\n\treturn Hello()\n}\n";

/// The test project's folder.
async fn folder(l: &Listening) -> PathBuf {
    let project = l.app.core.projects().get(&l.app.project).await.unwrap();
    PathBuf::from(project.directory.unwrap())
}

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// `assemble` starts no worker: a question then has no server to ask.
async fn start(l: &Listening) {
    l.app
        .core
        .codebase()
        .start(CodebaseConfig::default())
        .await
        .unwrap();
}

async fn shut_down(l: Listening) {
    l.app
        .core
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "needs gopls on PATH"]
async fn code_lsp_finds_references_with_gopls() {
    let l = listening_app().await;
    let dir = folder(&l).await;
    write(&dir, "go.mod", "module example.com/demo\n\ngo 1.21\n");
    write(&dir, "a.go", A_GO);
    write(&dir, "b.go", B_GO);
    start(&l).await;
    let (_, client) = project_client(&l).await;
    let args = json!({
        "operation": "findReferences", "file": "a.go", "line": 3, "character": 6,
    });
    let answer = ok(&client, "code_lsp", args).await;
    assert_eq!(answer["answer"], "answered", "{answer}");
    let paths: Vec<&str> = answer["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["path"].as_str().unwrap())
        .collect();
    assert!(
        paths.contains(&"a.go") && paths.contains(&"b.go"),
        "{answer}"
    );
    let at_b = answer["result"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["path"] == "b.go")
        .unwrap();
    assert_eq!(
        (at_b["line"].as_u64(), at_b["character"].as_u64()),
        (Some(4), Some(9))
    );
    shut_down(l).await;
}

#[tokio::test]
async fn code_lsp_answers_no_server_for_an_unknown_extension() {
    let l = listening_app().await;
    write(&folder(&l).await, "x.zz", "zz\n");
    let (_, client) = project_client(&l).await;
    let args = json!({ "operation": "hover", "file": "x.zz", "line": 1, "character": 1 });
    let answer = ok(&client, "code_lsp", args).await;
    assert_eq!(answer["answer"], "no_server", "{answer}");
    assert_eq!(answer["extension"], ".zz");
    assert!(answer["server"].is_null());
    shut_down(l).await;
}

#[tokio::test]
async fn code_lsp_refuses_a_path_outside_the_project() {
    let l = listening_app().await;
    let (_, client) = project_client(&l).await;
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "x.go", A_GO);
    let absolute = outside.path().join("x.go").to_string_lossy().into_owned();
    for file in ["../x.go".to_string(), absolute] {
        let args = json!({ "operation": "documentSymbol", "file": file });
        let text = refused(&client, "code_lsp", args).await;
        assert!(text.contains("INVALID_COMMAND"), "{file}: {text}");
    }
    shut_down(l).await;
}

#[tokio::test]
async fn code_lsp_refuses_a_link_out_of_the_project() {
    let l = listening_app().await;
    let dir = folder(&l).await;
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "x.go", A_GO);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let link = dir.join("src/out");
    #[cfg(windows)]
    {
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link.to_string_lossy().replace('/', "\\"))
            .arg(outside.path().to_string_lossy().replace('/', "\\"))
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    let (_, client) = project_client(&l).await;
    let args = json!({ "operation": "documentSymbol", "file": "src/out/x.go" });
    let text = refused(&client, "code_lsp", args).await;
    assert!(
        text.contains("the path must be inside the project"),
        "{text}"
    );
    shut_down(l).await;
}

/// A language server that answers `initialize` and nothing after it.
const SILENT_SERVER: &str = r#"
let buffer = Buffer.alloc(0);
process.stdin.on('data', (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  for (;;) {
    const end = buffer.indexOf('\r\n\r\n');
    if (end < 0) return;
    const length = Number(/Content-Length: (\d+)/i.exec(buffer.slice(0, end).toString())[1]);
    if (buffer.length < end + 4 + length) return;
    const message = JSON.parse(buffer.slice(end + 4, end + 4 + length).toString());
    buffer = buffer.slice(end + 4 + length);
    if (message.method === 'initialize') {
      const body = JSON.stringify({ jsonrpc: '2.0', id: message.id, result: { capabilities: {} } });
      process.stdout.write('Content-Length: ' + Buffer.byteLength(body) + '\r\n\r\n' + body);
    }
  }
});
"#;

#[tokio::test]
async fn a_slow_answer_times_out() {
    let l = listening_app().await;
    let scripts = tempfile::tempdir().unwrap();
    let script = scripts.path().join("silent.js");
    std::fs::write(&script, SILENT_SERVER).unwrap();
    let silent = AddedServer {
        id: "silent".into(),
        command: "node".into(),
        args: vec![script.to_string_lossy().into_owned()],
        extensions: vec![".slow".into()],
    };
    l.app
        .core
        .codebase()
        .add_server("add-silent".into(), silent)
        .await
        .unwrap();
    write(&folder(&l).await, "x.slow", "slow\n");
    start(&l).await;
    let (_, client) = project_client(&l).await;
    let args = json!({ "operation": "hover", "file": "x.slow", "line": 1, "character": 1 });
    let began = Instant::now();
    let (is_error, text) = call(&client, "code_lsp", args).await;
    assert!(is_error, "{text}");
    assert!(
        text.starts_with("LANGUAGE_SERVER_UNAVAILABLE: the language server silent is running")
            && text.contains("did not answer in time"),
        "{text}"
    );
    assert!(
        began.elapsed() < Duration::from_secs(25),
        "{:?}",
        began.elapsed()
    );
    shut_down(l).await;
}

#[test]
fn the_removed_code_tools_are_listed_nowhere() {
    for kind in [GrantKind::Thread, GrantKind::Project, GrantKind::Executor] {
        let tools = Grants::tools_for(kind);
        for gone in ["where_is", "who_uses", "outline"] {
            assert!(!tools.contains(&gone), "{kind:?} lists {gone}");
        }
        assert!(tools.contains(&"code_lsp"), "{kind:?} lacks code_lsp");
    }
}

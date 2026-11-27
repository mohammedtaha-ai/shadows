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

const LIB_TS: &str = "export function greet(name: string): string {\n  return \"hi \" + name;\n}\n";
const MAIN_TS: &str = concat!(
    "import { greet } from \"./lib\";\n\n",
    "export function run(): string {\n  return greet(\"x\");\n}\n",
);

/// Each location of an answer (`from` for a call), as (path, line).
fn places(answer: &serde_json::Value) -> Vec<(String, u64)> {
    answer["result"]
        .as_array()
        .unwrap_or_else(|| panic!("{answer}"))
        .iter()
        .map(|item| {
            if item["from"].is_object() {
                &item["from"]
            } else {
                item
            }
        })
        .map(|l| {
            (
                l["path"].as_str().unwrap_or("").to_owned(),
                l["line"].as_u64().unwrap_or(0),
            )
        })
        .collect()
}

#[tokio::test]
#[ignore = "needs TypeScript 7 (npm install -g typescript)"]
async fn code_lsp_answers_typescript_with_tsc() {
    let l = listening_app().await;
    let dir = folder(&l).await;
    write(&dir, "lib.ts", LIB_TS);
    write(&dir, "main.ts", MAIN_TS);
    // Without a tsconfig.json TypeScript knows only the open file and what it imports,
    // so the uses in main.ts are found only by chance (main.ts not yet closed).
    write(&dir, "tsconfig.json", "{}\n");
    start(&l).await;
    let (_, client) = project_client(&l).await;
    let ask = |args: serde_json::Value| {
        let client = &client;
        async move {
            let answer = ok(client, "code_lsp", args).await;
            assert_eq!(answer["answer"], "answered", "{answer}");
            assert_eq!(answer["server"], "typescript", "{answer}");
            places(&answer)
        }
    };
    let lib = |line: u64| ("lib.ts".to_owned(), line);
    let main = |line: u64| ("main.ts".to_owned(), line);

    let definition = json!({
        "operation": "goToDefinition", "file": "main.ts", "line": 4, "character": 10,
    });
    assert_eq!(ask(definition).await, vec![lib(1)]);

    let references = json!({
        "operation": "findReferences", "file": "lib.ts", "line": 1, "character": 17,
    });
    let mut found = ask(references).await;
    found.sort();
    assert_eq!(found, vec![lib(1), main(1), main(4)]);

    let symbols = ask(json!({ "operation": "documentSymbol", "file": "lib.ts" })).await;
    assert_eq!(symbols, vec![lib(1)]);

    let named = ask(json!({ "operation": "workspaceSymbol", "query": "greet" })).await;
    assert!(named.contains(&lib(1)), "{named:?}");

    let calls = json!({
        "operation": "incomingCalls", "file": "lib.ts", "line": 1, "character": 17,
    });
    assert_eq!(ask(calls).await, vec![main(3)]);
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

/// A language server for call hierarchies. Its arguments: the transcript file, where
/// each message's method is appended as it arrives, and a mode. `many` prepares 120
/// roots whose incoming call lies outside every project, except root 100's, which lies
/// in the opened file; `slow` prepares five roots and answers each follow-up after 8 s.
const HIERARCHY_SERVER: &str = r#"
const fs = require('fs');
const [transcript, mode] = process.argv.slice(2);
let buffer = Buffer.alloc(0);
let opened = '';
const send = (m) => {
  const body = JSON.stringify(Object.assign({ jsonrpc: '2.0' }, m));
  process.stdout.write('Content-Length: ' + Buffer.byteLength(body) + '\r\n\r\n' + body);
};
const range = { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } };
const handle = (m) => {
  if (m.method) fs.appendFileSync(transcript, m.method + '\n');
  if (m.method === 'initialize') send({ id: m.id, result: { capabilities: {} } });
  else if (m.method === 'textDocument/didOpen') opened = m.params.textDocument.uri;
  else if (m.method === 'textDocument/prepareCallHierarchy') {
    const roots = mode === 'many' ? 120 : 5;
    const items = [];
    for (let i = 1; i <= roots; i++) {
      items.push({ name: 'r' + i, kind: 12, uri: opened, range, selectionRange: range,
        data: i });
    }
    send({ id: m.id, result: items });
  } else if (m.method === 'callHierarchy/incomingCalls') {
    const i = m.params.item.data;
    const uri = i === 100 ? opened : 'file:///nowhere/far/away.calls';
    const call = [{ from: { name: 'caller' + i, kind: 12, uri, range, selectionRange: range },
      fromRanges: [range] }];
    if (mode === 'slow') setTimeout(() => send({ id: m.id, result: call }), 8000);
    else send({ id: m.id, result: call });
  } else if (m.id !== undefined) send({ id: m.id, result: null });
};
process.stdin.on('data', (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  for (;;) {
    const end = buffer.indexOf('\r\n\r\n');
    if (end < 0) return;
    const length = Number(/Content-Length: (\d+)/i.exec(buffer.slice(0, end).toString())[1]);
    if (buffer.length < end + 4 + length) return;
    const message = JSON.parse(buffer.slice(end + 4, end + 4 + length).toString());
    buffer = buffer.slice(end + 4 + length);
    handle(message);
  }
});
"#;

/// A project whose `.calls` files are read by `HIERARCHY_SERVER` in `mode`; returns
/// the transcript's path and the folder that holds it.
async fn hierarchy_server(l: &Listening, mode: &str) -> (PathBuf, tempfile::TempDir) {
    let scripts = tempfile::tempdir().unwrap();
    let script = scripts.path().join("hierarchy.js");
    let transcript = scripts.path().join("transcript.txt");
    std::fs::write(&script, HIERARCHY_SERVER).unwrap();
    let server = AddedServer {
        id: "hierarchy".into(),
        command: "node".into(),
        args: vec![
            script.to_string_lossy().into_owned(),
            transcript.to_string_lossy().into_owned(),
            mode.into(),
        ],
        extensions: vec![".calls".into()],
    };
    l.app
        .core
        .codebase()
        .add_server("add-hierarchy".into(), server)
        .await
        .unwrap();
    write(&folder(l).await, "x.calls", "calls\n");
    start(l).await;
    (transcript, scripts)
}

/// How many times the transcript names `method`, once `didClose` is in it.
async fn heard(transcript: &Path, method: &str) -> usize {
    for _ in 0..100 {
        let text = std::fs::read_to_string(transcript).unwrap_or_default();
        if text.contains("textDocument/didClose") {
            return text.lines().filter(|line| *line == method).count();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the document was never closed");
}

#[tokio::test]
async fn a_hierarchy_follows_up_at_most_a_hundred_roots_in_one_open_document() {
    let l = listening_app().await;
    let (transcript, _scripts) = hierarchy_server(&l, "many").await;
    let (_, client) = project_client(&l).await;
    let args = json!({
        "operation": "incomingCalls", "file": "x.calls", "line": 1, "character": 1,
    });
    let answer = ok(&client, "code_lsp", args).await;
    assert_eq!(answer["answer"], "answered", "{answer}");
    assert_eq!(answer["truncated"], true, "{answer}");
    let result = answer["result"].as_array().unwrap();
    assert_eq!(result.len(), 1, "{answer}");
    assert_eq!(result[0]["from"]["name"], "caller100", "{answer}");
    assert_eq!(result[0]["from"]["path"], "x.calls", "{answer}");
    assert_eq!(heard(&transcript, "textDocument/didClose").await, 1);
    assert_eq!(heard(&transcript, "textDocument/didOpen").await, 1);
    assert_eq!(heard(&transcript, "callHierarchy/incomingCalls").await, 100);
    shut_down(l).await;
}

#[tokio::test]
async fn slow_hierarchy_follow_ups_share_one_deadline() {
    let l = listening_app().await;
    let (transcript, _scripts) = hierarchy_server(&l, "slow").await;
    let (_, client) = project_client(&l).await;
    let args = json!({
        "operation": "incomingCalls", "file": "x.calls", "line": 1, "character": 1,
    });
    let began = Instant::now();
    let (is_error, text) = call(&client, "code_lsp", args).await;
    let took = began.elapsed();
    assert!(is_error, "{text}");
    assert!(
        text.starts_with("LANGUAGE_SERVER_UNAVAILABLE: the language server hierarchy is running")
            && text.contains("did not answer in time"),
        "{text}"
    );
    // Five roots at 8 s each would take 40 s; one deadline ends near 20 s.
    assert!(
        took >= Duration::from_secs(18) && took < Duration::from_secs(25),
        "{took:?}"
    );
    assert_eq!(heard(&transcript, "textDocument/didClose").await, 1);
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

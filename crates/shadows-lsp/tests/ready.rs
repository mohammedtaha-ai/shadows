//! A server answers once it has read its workspace, not as soon as it initialized.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use shadows_lsp::{Server, ServerSpec};

/// As pyright does: after `initialized` it reads its workspace for a while, and
/// answers references with nothing until then; it tells the client, with a request
/// of its own, when the workspace is read.
const READS_FIRST: &str = r#"
let buffer = Buffer.alloc(0);
let read = false;
const send = (m) => {
  const body = JSON.stringify(Object.assign({ jsonrpc: '2.0' }, m));
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
};
const handle = (m) => {
  if (m.method === 'initialize') {
    send({ id: m.id, result: { capabilities: { referencesProvider: true } } });
  } else if (m.method === 'initialized') {
    send({ id: 1000, method: 'workspace/configuration', params: { items: [{}] } });
    setTimeout(() => {
      read = true;
      send({ id: 1001, method: 'client/registerCapability', params: { registrations: [] } });
    }, 400);
  } else if (m.method === 'textDocument/references') {
    const range = { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } };
    send({ id: m.id, result: read ? [{ uri: 'file:///b.py', range }] : [] });
  } else if (m.id !== undefined && m.method !== undefined) {
    send({ id: m.id, result: null });
  }
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

#[tokio::test]
async fn a_started_server_has_read_its_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(
        ServerSpec {
            executable: PathBuf::from("node"),
            args: vec!["-e".into(), READS_FIRST.into()],
            root: dir.path().to_path_buf(),
            env: std::env::vars().collect(),
        },
        Duration::from_secs(20),
    )
    .await
    .expect("the server starts");
    let uses = server
        .request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": "file:///a.py" },
                "position": { "line": 0, "character": 6 },
                "context": { "includeDeclaration": false },
            }),
            Duration::from_secs(10),
        )
        .await
        .expect("an answer");
    assert_eq!(uses.as_array().map(Vec::len), Some(1), "{uses}");
    server.shutdown().await;
}

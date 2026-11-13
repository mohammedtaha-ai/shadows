//! A server's own requests are answered, so a server that waits for them goes on.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use shadows_lsp::{Server, ServerSpec};

/// After `initialized` it asks the client four requests in turn, as typescript-go
/// asks `client/registerCapability`, and holds every request of ours until the last
/// is answered; it then answers with the four answers it got.
const ASKS_FIRST: &str = r#"
let buffer = Buffer.alloc(0);
let next = 1000;
const pending = new Map();
const held = [];
let answers = null;
const send = (m) => {
  const body = JSON.stringify(Object.assign({ jsonrpc: '2.0' }, m));
  process.stdout.write(`Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`);
};
const ask = (method, params, then) => {
  const id = next++;
  pending.set(id, then);
  send({ id, method, params });
};
const handle = (m) => {
  if (m.method === undefined) {
    const then = pending.get(m.id);
    pending.delete(m.id);
    then(m);
  } else if (m.method === 'initialize') {
    send({ id: m.id, result: { capabilities: {} } });
  } else if (m.method === 'initialized') {
    ask('client/registerCapability', { registrations: [] }, (a) =>
      ask('window/workDoneProgress/create', { token: 't' }, (b) =>
        ask('workspace/configuration', { items: [{}, {}] }, (c) =>
          ask('some/unknown', {}, (d) => {
            answers = [a, b, c, d];
            held.splice(0).forEach((answer) => answer());
          }))));
  } else if (m.id !== undefined) {
    const answer = () => send({ id: m.id, result: answers });
    if (answers) answer(); else held.push(answer);
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
async fn a_servers_requests_are_answered() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::start(
        ServerSpec {
            executable: PathBuf::from("node"),
            args: vec!["-e".into(), ASKS_FIRST.into()],
            root: dir.path().to_path_buf(),
            env: std::env::vars().collect(),
        },
        Duration::from_secs(20),
    )
    .await
    .expect("the server starts");
    let answers = server
        .request(
            "workspace/symbol",
            json!({ "query": "x" }),
            Duration::from_secs(10),
        )
        .await
        .expect("an answer once its own requests were answered");
    let results: Vec<&Value> = answers
        .as_array()
        .unwrap_or_else(|| panic!("{answers}"))
        .iter()
        .map(|a| {
            assert!(a.get("error").is_none(), "{a}");
            &a["result"]
        })
        .collect();
    assert_eq!(
        results,
        [
            &Value::Null,
            &Value::Null,
            &json!([null, null]),
            &Value::Null
        ]
    );
    server.shutdown().await;
}

//! A server that fails while it starts is reported, and leaves no panic behind.
//! Its own test binary: the panic hook counts every panic in it.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use shadows_lsp::{LspError, Server, ServerSpec};

static PANICS: AtomicUsize = AtomicUsize::new(0);

/// Answers every request with an error and keeps running, as
/// typescript-language-server does when it finds no TypeScript.
const REFUSES: &str = r#"
let seen = '';
process.stdin.on('data', (chunk) => {
  seen += chunk;
  const id = seen.match(/"id":(\d+)/);
  if (!id) return;
  seen = '';
  const answer = JSON.stringify({
    jsonrpc: '2.0', id: Number(id[1]), error: { code: -32603, message: 'no TypeScript' },
  });
  process.stdout.write(`Content-Length: ${Buffer.byteLength(answer)}\r\n\r\n${answer}`);
});
"#;

fn count_panics() {
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        let before = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            PANICS.fetch_add(1, Ordering::SeqCst);
            before(info);
        }));
    });
}

async fn start(executable: &str, args: &[&str]) -> Result<Server, LspError> {
    let dir = tempfile::tempdir().unwrap();
    Server::start(
        ServerSpec {
            executable: PathBuf::from(executable),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            root: dir.path().to_path_buf(),
            env: std::env::vars().collect(),
        },
        Duration::from_secs(20),
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_server_that_refuses_initialize_fails_with_no_panic() {
    count_panics();
    let started = start("node", &["-e", REFUSES]).await;
    match started {
        Err(LspError::Initialize(message)) => assert_eq!(message, "no TypeScript"),
        other => panic!("{:?}", other.err()),
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(PANICS.load(Ordering::SeqCst), 0, "a task panicked");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_server_that_exits_at_once_fails_with_no_panic() {
    count_panics();
    let started = if cfg!(windows) {
        start("cmd", &["/c", "exit 3"]).await
    } else {
        start("sh", &["-c", "exit 3"]).await
    };
    assert!(
        matches!(started, Err(LspError::Exited(_))),
        "{:?}",
        started.err()
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(PANICS.load(Ordering::SeqCst), 0, "a task panicked");
}

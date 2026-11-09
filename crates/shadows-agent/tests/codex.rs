use shadows_agent::acp::{McpServerSpec, PromptBlock, SessionSetup, SessionStart, TurnEnd};
use shadows_agent::codex::{CodexProgram, Connection};
use shadows_agent::connection::ConnectionError;
use shadows_agent::events::HarnessEvent;
use shadows_process::ProcessHandle;
use std::{path::PathBuf, sync::OnceLock, time::Duration};
use tokio::sync::mpsc;

/// The fake is built with `test-support` on demand, since `cargo test` alone
/// never builds a binary whose required feature no test crate enables.
fn fixture_executable() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fake-codex/Cargo.toml");
        escargot::CargoBuild::new()
            .manifest_path(manifest)
            .bin("fake-codex")
            .features("test-support")
            .run()
            .expect("build fake-codex with test-support")
            .path()
            .to_owned()
    })
    .clone()
}

struct Peer {
    _process: ProcessHandle,
    connection: Connection,
    thread: String,
    events: mpsc::UnboundedReceiver<HarnessEvent>,
}

impl Peer {
    async fn start() -> Self {
        let executable = fixture_executable();
        let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let setup = SessionSetup {
            mcp: vec![McpServerSpec::Http {
                name: "shadows".into(),
                url: "http://127.0.0.1:1/mcp".into(),
                headers: vec![("Authorization".into(), "Bearer fixture".into())],
            }],
            ..SessionSetup::default()
        };
        let program = CodexProgram::new(executable, cwd.clone());
        let spec = program.process_spec(&cwd, &setup).unwrap();
        let mut process = shadows_process::spawn(spec).expect("build fake-codex with test-support");
        let (send, events) = mpsc::unbounded_channel();
        let connection = Connection::open(&mut process, move |e| {
            let _ = send.send(e);
        })
        .await
        .unwrap();
        let opened = connection
            .start_session(&cwd, SessionStart::New, &setup, &["workspace_get"])
            .await
            .unwrap();
        Self {
            _process: process,
            connection,
            thread: opened.session_id,
            events,
        }
    }

    async fn prompt(&self, text: &str) -> Result<TurnEnd, ConnectionError> {
        self.connection
            .prompt_blocks(&self.thread, &[PromptBlock::Text(text.into())])
            .await
            .map(|result| result.end)
    }

    fn chunks(&mut self) -> Vec<String> {
        std::iter::from_fn(|| self.events.try_recv().ok())
            .filter_map(|e| match e {
                HarnessEvent::Chunk { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }
}

#[tokio::test]
async fn codex_streams_before_completing() {
    let mut peer = Peer::start().await;
    assert_eq!(peer.prompt("stream").await.unwrap(), TurnEnd::Ended);
    assert_eq!(peer.chunks(), ["first", "last"]);
}

#[tokio::test]
async fn codex_interrupt_waits_for_the_matching_terminal_event() {
    let mut peer = Peer::start().await;
    let connection = peer.connection.clone();
    let thread = peer.thread.clone();
    let mut turn = tokio::spawn(async move {
        connection
            .prompt_blocks(&thread, &[PromptBlock::Text("interrupt".into())])
            .await
    });
    let event = tokio::time::timeout(Duration::from_secs(3), peer.events.recv())
        .await
        .unwrap();
    assert!(matches!(event, Some(HarnessEvent::Chunk { text, .. }) if text == "waiting"));
    peer.connection.cancel(&peer.thread);
    assert!(
        tokio::time::timeout(Duration::from_millis(70), &mut turn)
            .await
            .is_err()
    );
    let result = tokio::time::timeout(Duration::from_secs(3), turn)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.end, TurnEnd::Cancelled);
}

#[tokio::test]
async fn codex_eof_fails_pending_work() {
    let peer = Peer::start().await;
    assert!(matches!(
        peer.prompt("rpc error").await,
        Err(ConnectionError::Rpc(_))
    ));
    assert!(matches!(
        peer.prompt("eof").await,
        Err(ConnectionError::Closed)
    ));
}

#[tokio::test]
async fn codex_unknown_notifications_are_ignored() {
    let mut peer = Peer::start().await;
    assert_eq!(
        peer.prompt("unknown notification").await.unwrap(),
        TurnEnd::Ended
    );
    assert_eq!(peer.chunks(), ["first", "last"]);
}

#[tokio::test]
async fn codex_server_requests_do_not_deadlock() {
    let mut peer = Peer::start().await;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), peer.prompt("approval"))
            .await
            .unwrap()
            .unwrap(),
        TurnEnd::Ended
    );
    assert_eq!(peer.chunks(), ["refused"]);
}

#[tokio::test]
async fn codex_cannot_finish_a_replacement_generation() {
    let mut peer = Peer::start().await;
    assert_eq!(
        peer.prompt("foreign terminal").await.unwrap(),
        TurnEnd::Ended
    );
    assert_eq!(peer.chunks(), ["current"]);
}

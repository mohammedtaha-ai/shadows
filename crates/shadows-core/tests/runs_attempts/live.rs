//! Attempts on the real Claude adapter, against MiniMax through a Claude
//! profile, in a real git repository on disk. CI skips them; run them with
//! `API_KEY_MINIMAX` set and `-- --ignored --test-threads=1`.
use super::*;
use shadows_agent::profile::ClaudeProfile;

const ADAPTER_VERSION: &str = "0.101.0";
const CLAUDE_VERSION: &str = "2.1.319";
const MINIMAX_URL: &str = "https://api.minimax.io/anthropic";
const MODEL: &str = "MiniMax-M3";
/// What the person picks: the adapter offers its own aliases, which the
/// profile maps to `MODEL`.
const PICKED: &str = "default";

/// The real adapter on MiniMax. The daemon passes an adapter only `PATH` and
/// a few names, so a launcher next to it sets the profile's environment and
/// then runs the pinned adapter itself. The key stays in `dir`, which the
/// test deletes.
struct LiveAdapter {
    adapter: Arc<ClaudeAdapter>,
    _dir: tempfile::TempDir,
}

fn live_adapter() -> LiveAdapter {
    let key = std::env::var("API_KEY_MINIMAX").expect("API_KEY_MINIMAX is required");
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join(".claude.json"),
        r#"{"hasCompletedOnboarding":true}"#,
    )
    .unwrap();
    let profile = ClaudeProfile::new(
        "executor-live".into(),
        home,
        [
            ("ANTHROPIC_BASE_URL", MINIMAX_URL),
            ("ANTHROPIC_MODEL", MODEL),
            ("ANTHROPIC_DEFAULT_HAIKU_MODEL", MODEL),
            ("ANTHROPIC_DEFAULT_SONNET_MODEL", MODEL),
            ("ANTHROPIC_DEFAULT_OPUS_MODEL", MODEL),
            ("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "2048"),
            ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
        ]
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .to_vec(),
    );
    let env: serde_json::Map<String, Value> = profile
        .spawn_env_with(&[], Some(&key))
        .unwrap()
        .into_iter()
        .map(|(name, value)| (name, Value::from(value)))
        .collect();
    std::fs::write(dir.path().join("env.json"), Value::Object(env).to_string()).unwrap();
    let pinned = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../harness/claude/node_modules/@agentclientprotocol")
        .join("claude-agent-acp/dist/index.js");
    let pinned = std::fs::canonicalize(pinned).unwrap();
    let launcher = dir.path().join("launch.mjs");
    std::fs::write(
        &launcher,
        format!(
            "import {{ readFileSync }} from 'node:fs';\n\
             import {{ pathToFileURL }} from 'node:url';\n\
             const env = JSON.parse(readFileSync(new URL('./env.json', import.meta.url)));\n\
             Object.assign(process.env, env);\n\
             await import(pathToFileURL({}).href);\n",
            Value::from(pinned.to_string_lossy().into_owned())
        ),
    )
    .unwrap();
    let adapter = ClaudeAdapter::new(
        r"C:\Program Files\nodejs\node.exe".into(),
        launcher,
        r"C:\Users\Mohammed\.local\bin\claude.exe".into(),
        ADAPTER_VERSION.into(),
        CLAUDE_VERSION.into(),
    );
    LiveAdapter {
        adapter: Arc::new(adapter),
        _dir: dir,
    }
}

async fn live_fixture() -> (Fixture, LiveAdapter) {
    let live = live_adapter();
    let f = fixture_with(true, Some(live.adapter.clone())).await;
    (f, live)
}

impl Fixture {
    async fn start_live(&self, command: &str, task: u32, prompt: &str) -> Attempt {
        self.core.runs().set_first_prompt_for_tests(prompt);
        let attempt = self
            .core
            .runs()
            .start(command.into(), &self.workflow, 1, task, PICKED.into())
            .await
            .unwrap();
        assert_eq!(attempt.state, AttemptState::Running, "{:?}", attempt.reason);
        attempt
    }

    /// Waits up to three minutes for a real turn to end.
    async fn settled_live(&self, attempt: &Attempt) -> Attempt {
        for _ in 0..1800 {
            let now = self.state_of(attempt).await;
            if now.state != AttemptState::Running {
                return now;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the live attempt stayed running");
    }

    async fn entries_of(&self, thread: &ThreadId) -> Vec<shadows_core::ThreadEntry> {
        shadows_core::Threads::list_thread_entries_for_test(&self.storage, thread)
            .await
            .unwrap()
    }
}

#[tokio::test]
#[ignore = "live: needs API_KEY_MINIMAX"]
async fn live_an_executor_writes_inside_its_task_in_mode_default_and_is_refused_outside() {
    let (f, _live) = live_fixture().await;
    let attempt = f
        .start_live(
            "live-1",
            3,
            "Use the Write tool to create backend/app/inside.txt containing the word A. \
             Then use the Write tool to create web/outside.txt containing the word B. \
             Do not use Bash. Then stop.",
        )
        .await;
    let offered = f.sessions.offered(&attempt.thread_id).await.unwrap();
    assert_eq!(offered.current.mode, "default");
    let ended = f.settled_live(&attempt).await;
    let entries = f.entries_of(&attempt.thread_id).await;
    let kinds: Vec<_> = entries.iter().map(|e| e.kind).collect();
    println!(
        "live write boundary: {:?} {:?} {kinds:?}",
        ended.state, ended.reason
    );
    assert!(
        f.root.join("backend/app/inside.txt").is_file(),
        "the write inside `writes` did not run"
    );
    assert!(
        !f.root.join("web/outside.txt").exists(),
        "a write outside `writes` reached the folder"
    );
    assert!(
        kinds.contains(&ThreadEntryKind::PermissionRefused),
        "no refusal recorded: {kinds:?}"
    );
    // What git sees is the task's file and nothing outside it.
    let status = git(&f.root, &["status", "--porcelain", "--untracked-files=all"]);
    println!("git status: {status:?}");
    assert!(status.contains("backend/app/inside.txt"), "{status}");
    assert!(!status.contains("web/"), "{status}");
    assert_eq!(ended.reason.as_deref(), Some("ended without a commit"));
    f.finish().await;
}

/// An executor's write boundary holds only in mode `default`: in any other
/// mode its edits would not reach Shadows to be refused. On a real executor
/// session, a turn in another mode is refused before anything is written.
#[tokio::test]
#[ignore = "live: needs API_KEY_MINIMAX"]
async fn live_an_executor_turn_in_another_mode_is_refused() {
    let (f, _live) = live_fixture().await;
    let project = f.project.clone();
    let thread = f
        .storage
        .write_txn(move |conn| {
            Box::pin(async move {
                shadows_core::Threads::create_executor_in(
                    conn,
                    &project,
                    "T5".into(),
                    "claude-code",
                )
                .await
            })
        })
        .await
        .unwrap();
    let scope = shadows_core::testing::TaskScope {
        workflow: SOURCE.into(),
        version: 1,
        task: 5,
    };
    f.core
        .harness()
        .open_executor(&thread, &["web/**".into()], scope, PICKED.into())
        .await
        .unwrap();
    // The modes this session offers besides `default`; Shadows offers no others.
    for mode in ["acceptEdits", "auto"] {
        let sent = f
            .core
            .turns()
            .send_with_default_effort(
                thread.clone(),
                shadows_core::SendTurn {
                    command_id: format!("mode-{mode}"),
                    prompt: "Use the Write tool to create web/x.txt containing A.".into(),
                    attachments: vec![],
                    model: PICKED.into(),
                    mode: mode.into(),
                    effort: None,
                    focus: None,
                    plan: None,
                    client_tab: None,
                },
            )
            .await;
        println!("live mode {mode}: {sent:?}");
        assert!(
            matches!(sent, Err(shadows_core::TurnsError::ModeNotAllowed(ref m)) if m == mode),
            "{sent:?}"
        );
    }
    let offered = f.sessions.offered(&thread).await.unwrap();
    assert_eq!(offered.current.mode, "default");
    assert!(f.entries_of(&thread).await.is_empty());
    assert!(!f.root.join("web/x.txt").exists());
    f.core.harness().release_executor(&thread).await.unwrap();
    f.finish().await;
}

#[tokio::test]
#[ignore = "live: needs API_KEY_MINIMAX"]
async fn live_stop_ends_a_running_attempt_and_frees_its_scope() {
    let (f, _live) = live_fixture().await;
    let attempt = f
        .start_live(
            "live-2",
            5,
            "Run the Bash command `sleep 60` and then reply with the word DONE.",
        )
        .await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let stopped = f.core.runs().stop(&attempt.id).await.unwrap();
    println!("live stop: {:?} {:?}", stopped.state, stopped.reason);
    assert_eq!(stopped.state, AttemptState::Stopped);
    assert!(stopped.ended_at.is_some());
    assert_eq!(f.sessions.live_count().await, 0);
    let held: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM run_attempts WHERE id = ? AND released_at IS NULL",
    )
    .bind(attempt.id.as_str())
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(held, 0);
    let live: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM mcp_grant WHERE kind = 'executor' AND revoked_at IS NULL",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(live, 0);
    f.finish().await;
}

#[tokio::test]
#[ignore = "live: needs API_KEY_MINIMAX"]
async fn live_an_executors_first_turn_hears_no_planner_instructions() {
    let (f, _live) = live_fixture().await;
    f.core
        .instructions()
        .save(
            "live-i1".into(),
            &f.project,
            "End every reply with the word PINEAPPLE.",
        )
        .await
        .unwrap();
    let attempt = f
        .start_live(
            "live-3",
            1,
            "Reply with the single word OK. Do not use any tool.",
        )
        .await;
    f.settled_live(&attempt).await;
    let entries = f.entries_of(&attempt.thread_id).await;
    let reply = entries
        .iter()
        .rev()
        .find(|e| e.kind == ThreadEntryKind::AgentMessage)
        .expect("an agent reply");
    println!("live reply: {:?}", reply.body);
    assert!(reply.body.contains("OK"), "{}", reply.body);
    assert!(!reply.body.contains("PINEAPPLE"), "{}", reply.body);
    let sent = entries
        .iter()
        .find(|e| e.kind == ThreadEntryKind::UserMessage)
        .expect("the first prompt");
    assert!(!sent.body.contains("PINEAPPLE"), "{}", sent.body);
    // No planning context was carried to the executor.
    let carried: i64 =
        sqlx::query_scalar("SELECT count(*) FROM invocation_context WHERE thread_id = ?")
            .bind(attempt.thread_id.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(carried, 0);
    f.finish().await;
}

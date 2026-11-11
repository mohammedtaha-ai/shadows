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
    let env = profile.spawn_env_with(&[], Some(&key)).unwrap();
    launched(dir, env)
}

/// The pinned adapter run by a launcher in `dir` that first sets `env`.
fn launched(dir: tempfile::TempDir, env: Vec<(String, String)>) -> LiveAdapter {
    let env: serde_json::Map<String, Value> = env
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

/// The model picked on the subscription.
const SUBSCRIPTION_MODEL: &str = "sonnet";

/// The real adapter on the person's signed-in Claude Code, with no provider
/// override: `auto`'s safety check runs on Anthropic's server, which MiniMax
/// does not pass through, so there `auto` refuses every `Bash` call. The
/// launcher hands the adapter this process's environment, without any
/// `ANTHROPIC_` name or `CLAUDE_CONFIG_DIR`, so Claude Code reads its own
/// sign-in from the user's home.
async fn subscription_fixture() -> (Fixture, LiveAdapter) {
    let env = std::env::vars()
        .filter(|(name, _)| {
            !name.starts_with("ANTHROPIC_")
                && !name.starts_with("API_KEY_")
                && name != "CLAUDE_CONFIG_DIR"
        })
        .collect();
    let live = launched(tempfile::tempdir().unwrap(), env);
    let f = fixture_with(true, Some(live.adapter.clone())).await;
    (f, live)
}

impl Fixture {
    async fn start_live(&self, command: &str, task: u32, prompt: &str) -> Attempt {
        self.start_live_on(PICKED, command, task, prompt).await
    }

    async fn start_live_on(&self, model: &str, command: &str, task: u32, prompt: &str) -> Attempt {
        self.core.runs().set_first_prompt_for_tests(prompt);
        let attempt = self
            .core
            .runs()
            .start(command.into(), &self.workflow, 1, task, model.into())
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

/// In `auto`, the executor changes a file that already exists in a subfolder
/// with `Edit`, nothing is refused, and the commit it makes is accepted. No
/// `/mcp` is bound here, so the test reports that commit, as the session's
/// grant would, while the turn still runs. A commit outside `writes` is refused
/// by `a_commit_with_paths_outside_writes_is_refused_until_fixed`.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_an_executor_edits_a_file_in_a_subfolder_in_mode_auto_and_its_commit_is_accepted() {
    let (f, _live) = subscription_fixture().await;
    let path = "backend/app/Http/routes.php";
    std::fs::create_dir_all(f.root.join("backend/app/Http")).unwrap();
    std::fs::write(
        f.root.join(path),
        "<?php\n\nnamespace App\\Http;\n\nuse App\\Models\\User;\n// ROUTES\n",
    )
    .unwrap();
    git(&f.root, &["add", path]);
    git(&f.root, &["commit", "-q", "-m", "routes"]);
    let start = git(&f.root, &["rev-parse", "HEAD"]).trim().to_string();
    let attempt = f
        .start_live_on(
            SUBSCRIPTION_MODEL,
            "live-1",
            3,
            &format!(
                "Use the Edit tool to replace the line `// ROUTES` in {path} with the line \
                 `use App\\Http\\Controllers\\HomeController;`. Change nothing else. Then \
                 commit only that file with git: `git add {path}`, then \
                 `git commit -m \"Use the home controller\"`. Then run the Bash command \
                 `sleep 30`, and stop."
            ),
        )
        .await;
    let offered = f.sessions.offered(&attempt.thread_id).await.unwrap();
    assert_eq!(offered.current.mode, "auto");
    let mut commit = None;
    for _ in 0..1800 {
        let head = git(&f.root, &["rev-parse", "HEAD"]).trim().to_string();
        if head != start {
            commit = Some(head);
            break;
        }
        if f.state_of(&attempt).await.state != AttemptState::Running {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let entries = f.entries_of(&attempt.thread_id).await;
    let kinds: Vec<_> = entries.iter().map(|e| e.kind).collect();
    println!("live edit in auto: {commit:?} {kinds:?}");
    let commit = commit.expect("the executor made no commit");
    // This fixture binds no `/mcp`, so its sessions hold no grant row: the
    // grant `task_report` would carry names the attempt's thread and task.
    let grant = shadows_core::Grant {
        id: shadows_core::GrantId::generate(),
        kind: shadows_core::GrantKind::Executor,
        project_id: f.project.clone(),
        thread_id: Some(attempt.thread_id.clone()),
        created_at: "2026-11-11T00:00:00Z".into(),
        revoked_at: None,
        scope: Some(shadows_core::testing::TaskScope {
            workflow: SOURCE.into(),
            version: 1,
            task: 3,
        }),
    };
    let done = f
        .core
        .runs()
        .report(&grant, &commit, "Edited the routes file.".into())
        .await
        .unwrap();
    assert_eq!(done.state, AttemptState::Committed);
    assert_eq!(done.commit.as_deref(), Some(commit.as_str()));
    let names = git(&f.root, &["show", "--name-only", "--format=", &commit]);
    assert_eq!(names.trim(), path, "{names}");
    // The file tool keeps the namespace's backslashes the shell dropped.
    let written = git(&f.root, &["show", &format!("{commit}:{path}")]);
    assert!(
        written.contains("use App\\Http\\Controllers\\HomeController;"),
        "{written}"
    );
    assert!(written.contains("namespace App\\Http;"), "{written}");
    assert!(!written.contains("// ROUTES"), "{written}");
    assert!(
        !kinds.contains(&ThreadEntryKind::PermissionRefused),
        "a refusal in `auto`: {kinds:?}"
    );
    f.finish().await;
}

/// An executor runs in `auto` and in no other mode, whatever its project
/// allows. On a real executor session, a turn in another mode is refused
/// before anything is written.
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
        .open_executor(&thread, scope, PICKED.into())
        .await
        .unwrap();
    // A mode the project allows, and one Shadows offers no thread.
    for mode in ["acceptEdits", "default"] {
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
            matches!(
                sent,
                Err(shadows_core::TurnsError::ModeNotAllowed(ref m))
                    | Err(shadows_core::TurnsError::SettingNotOffered { id: ref m, .. })
                    if m == mode
            ),
            "{sent:?}"
        );
    }
    let offered = f.sessions.offered(&thread).await.unwrap();
    assert_eq!(offered.current.mode, "auto");
    assert!(f.entries_of(&thread).await.is_empty());
    assert!(!f.root.join("web/x.txt").exists());
    f.core.harness().release_executor(&thread).await.unwrap();
    f.finish().await;
}

/// Stop while Start is still opening the real session: Start sends no turn on
/// the stopped attempt, and no session is left open.
#[tokio::test]
#[ignore = "live: needs API_KEY_MINIMAX"]
async fn live_stop_while_the_session_opens_sends_no_turn() {
    let (f, _live) = live_fixture().await;
    f.core
        .runs()
        .set_first_prompt_for_tests("Use the Write tool to create web/late.txt containing A.");
    let core = f.core.clone();
    let workflow = f.workflow.clone();
    let starting = tokio::spawn(async move {
        core.runs()
            .start("live-open".into(), &workflow, 1, 5, PICKED.into())
            .await
    });
    let attempt = loop {
        let listed = f.core.runs().attempts(&f.workflow, 1, 5).await.unwrap();
        if let Some(attempt) = listed.into_iter().next() {
            break attempt;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    };
    assert!(
        attempt.operation_id.is_none(),
        "the turn was sent before Stop"
    );
    let stopped = f.core.runs().stop(&attempt.id).await.unwrap();
    assert_eq!(stopped.state, AttemptState::Stopped);
    let started = starting.await.unwrap().unwrap();
    println!(
        "live stop while opening: {:?} {:?}",
        started.state, started.reason
    );
    assert_eq!(started.state, AttemptState::Stopped);
    assert!(
        started.operation_id.is_none(),
        "a turn was sent: {started:?}"
    );
    let operations = f.core.turns().operations(&attempt.thread_id).await.unwrap();
    assert!(operations.is_empty(), "{operations:?}");
    for _ in 0..100 {
        if f.sessions.live_count().await == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(f.sessions.live_count().await, 0);
    assert!(!f.root.join("web/late.txt").exists());
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

//! Project decisions: the Planner proposes a revision on its thread, the
//! person accepts it, and a newer accepted revision supersedes the older.
//! The Planner's turn runs on `fake_acp`, held open by its "hang" prompt.

use std::sync::Arc;

use shadows_core::testing::{LiveHandles, Storage, acp};
use shadows_core::{
    AppCore, CoreError, CoreParts, DecisionInput, DecisionRecord, DecisionState, DesignError,
    ErrorCode, Grant, GrantId, GrantKind, ProjectId, SendTurn,
};

struct Fixture {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    project: ProjectId,
    grant: Grant,
    _tmp: tempfile::TempDir,
}

fn input(title: &str, id: Option<&str>, expected: Option<i64>) -> DecisionInput {
    DecisionInput {
        id: id.map(Into::into),
        expected_version: expected,
        title: title.into(),
        decision: "Use SQLx for every query.".into(),
        rationale: "One library, checked queries.".into(),
        source: "the kickoff call".into(),
    }
}

impl Fixture {
    async fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let storage = Arc::new(Storage::open(&tmp.path().join("d.sqlite3")).await.unwrap());
        let runtime = Arc::new(AppCore::runtime_for_tests(storage.clone()).await.unwrap().0);
        let core = AppCore::assemble(CoreParts {
            operations: AppCore::operations_for_tests(storage.clone()),
            secrets: shadows_core::testing::secrets(storage.clone(), None),
            storage: storage.clone(),
            runtime,
            sessions: acp::fake_sessions(storage.clone()),
            handles: Arc::new(LiveHandles::default()),
            bus: tokio::sync::broadcast::channel(32).0,
            ui: tokio::sync::broadcast::channel(32).0,
            mcp_url: acp::MCP_URL.into(),
        });
        let project = core
            .project_lifecycle()
            .create(
                "project".into(),
                "decisions",
                "Decisions",
                tmp.path().to_str().unwrap(),
            )
            .await
            .unwrap()
            .id;
        let thread = core
            .threads()
            .create("thread".into(), &project, "Planner", None, None)
            .await
            .unwrap();
        core.turns()
            .send(
                thread.id.clone(),
                SendTurn {
                    attachments: Vec::new(),
                    command_id: "turn".into(),
                    prompt: "hang".into(),
                    model: "fake-large".into(),
                    mode: "acceptEdits".into(),
                    effort: Some("high".into()),
                    focus: None,
                    plan: None,
                    client_tab: None,
                },
            )
            .await
            .unwrap();
        let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&storage, &thread.id)
            .await
            .unwrap();
        Self {
            core,
            storage,
            project,
            grant,
            _tmp: tmp,
        }
    }

    async fn propose(&self, cmd: &str, i: DecisionInput) -> Result<DecisionRecord, DesignError> {
        self.core
            .design()
            .propose_decision(&self.grant, cmd.into(), i)
            .await
    }

    async fn accept(&self, cmd: &str, id: &str, v: i64) -> Result<DecisionRecord, DesignError> {
        self.core
            .design()
            .accept_decision(cmd.into(), &self.project, id, v)
            .await
    }

    async fn count(&self, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(self.storage.reader())
            .await
            .unwrap()
    }

    async fn close(self) {
        self.core
            .shut_down(std::time::Duration::from_secs(3), std::future::pending())
            .await
            .unwrap();
        self.storage.close().await.unwrap();
    }
}

fn code(error: DesignError) -> ErrorCode {
    match CoreError::from(error) {
        CoreError::Refused { code, .. } => code,
        other => panic!("not a refusal: {other:?}"),
    }
}

#[tokio::test]
async fn the_planner_proposes_and_the_person_accepts() {
    let f = Fixture::new().await;
    let proposed = f.propose("p1", input("Queries", None, None)).await.unwrap();
    assert_eq!((proposed.id.as_str(), proposed.version), ("D1", 1));
    assert_eq!(proposed.state, DecisionState::Proposed);
    let cards: Vec<String> = sqlx::query_scalar(
        "SELECT card_json FROM thread_entry WHERE kind = 'DecisionProposal' AND thread_id = ?",
    )
    .bind(f.grant.thread_id.as_ref().unwrap().as_str())
    .fetch_all(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(cards.len(), 1);
    let card: DecisionRecord = serde_json::from_str(&cards[0]).unwrap();
    assert_eq!(card, proposed);
    assert_eq!(
        f.count("SELECT count(*) FROM durable_event WHERE kind = 'DecisionProposed'")
            .await,
        1
    );
    let accepted = f.accept("a1", "D1", 1).await.unwrap();
    assert_eq!(accepted.state, DecisionState::Accepted);
    let by = accepted.approved_by.unwrap();
    assert_eq!(
        (by.principal.as_str(), by.agent.as_str()),
        ("user:local", "human")
    );
    assert_eq!(
        f.count("SELECT count(*) FROM durable_event WHERE kind = 'DecisionAccepted'")
            .await,
        1
    );
    let listed = f.core.design().decisions(&f.project).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].state, DecisionState::Accepted);
    f.close().await;
}

#[tokio::test]
async fn a_new_version_supersedes_the_accepted_one() {
    let f = Fixture::new().await;
    f.propose("p1", input("Queries", None, None)).await.unwrap();
    f.accept("a1", "D1", 1).await.unwrap();
    let next = f
        .propose("p2", input("Queries, revised", Some("D1"), Some(1)))
        .await
        .unwrap();
    assert_eq!((next.id.as_str(), next.version), ("D1", 2));
    f.accept("a2", "D1", 2).await.unwrap();
    let states: Vec<_> = (f.core.design().decisions(&f.project).await.unwrap())
        .into_iter()
        .map(|d| (d.version, d.state))
        .collect();
    assert_eq!(
        states,
        vec![(1, DecisionState::Superseded), (2, DecisionState::Accepted)]
    );
    assert_eq!(
        f.count("SELECT count(*) FROM durable_event WHERE kind = 'DecisionAccepted'")
            .await,
        2,
        "superseding adds no event of its own"
    );
    f.close().await;
}

#[tokio::test]
async fn a_stale_or_invalid_decision_is_refused() {
    let f = Fixture::new().await;
    f.propose("p1", input("Queries", None, None)).await.unwrap();
    let stale = f.propose("p2", input("Again", Some("D1"), Some(3))).await;
    assert_eq!(code(stale.unwrap_err()), ErrorCode::DecisionStale);
    let empty = f.propose("p3", input("   ", None, None)).await;
    assert_eq!(code(empty.unwrap_err()), ErrorCode::InvalidDecision);
    f.propose("p4", input("Again", Some("D1"), Some(1)))
        .await
        .unwrap();
    let old = f.accept("a1", "D1", 1).await;
    assert_eq!(code(old.unwrap_err()), ErrorCode::DecisionStale);
    let listed = f.core.design().decisions(&f.project).await.unwrap();
    assert_eq!(listed.len(), 2, "a refusal writes nothing");
    f.close().await;
}

#[tokio::test]
async fn only_a_planner_proposes() {
    let f = Fixture::new().await;
    let executor = Grant {
        id: GrantId::generate(),
        kind: GrantKind::Executor,
        project_id: f.project.clone(),
        thread_id: f.grant.thread_id.clone(),
        created_at: "2026-11-16T00:00:00Z".into(),
        revoked_at: None,
        scope: Some(shadows_core::testing::TaskScope {
            workflow: "w".into(),
            version: 1,
            task: 1,
        }),
    };
    let refused = f
        .core
        .design()
        .propose_decision(&executor, "x1".into(), input("Queries", None, None))
        .await;
    assert!(refused.is_err(), "{refused:?}");
    assert!(
        f.core
            .design()
            .decisions(&f.project)
            .await
            .unwrap()
            .is_empty()
    );
    for kind in [GrantKind::Thread, GrantKind::Project, GrantKind::Executor] {
        let tools = shadows_core::Grants::tools_for(kind);
        assert!(
            !tools.iter().any(|t| t.starts_with("decision_accept")),
            "{kind:?}"
        );
    }
    f.close().await;
}

#[tokio::test]
async fn a_decision_replay_returns_the_original_revision() {
    let f = Fixture::new().await;
    let first = f.propose("p1", input("Queries", None, None)).await.unwrap();
    let again = f.propose("p1", input("Queries", None, None)).await.unwrap();
    assert_eq!(first, again);
    let accepted = f.accept("a1", "D1", 1).await.unwrap();
    assert_eq!(f.accept("a1", "D1", 1).await.unwrap(), accepted);
    assert_eq!(
        f.core.design().decisions(&f.project).await.unwrap().len(),
        1
    );
    assert_eq!(
        f.count("SELECT count(*) FROM thread_entry WHERE kind = 'DecisionProposal'")
            .await,
        1
    );
    f.close().await;
}

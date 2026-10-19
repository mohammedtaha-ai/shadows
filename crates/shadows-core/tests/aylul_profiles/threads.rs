//! AYLUL threads behavior.
use super::*;

#[tokio::test]
async fn aylul_harness_only_in_the_aylul_project() {
    let w = seeded().await;
    let core = core(&w).await;
    w.profiles
        .create_agent("agent".into(), agent("Work"))
        .await
        .unwrap();
    let normal = core
        .projects()
        .create(
            "normal".into(),
            "normal",
            "Normal",
            w._tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let assistant = core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &w._tmp.path().join("home"))
        .await
        .unwrap();
    assert!(
        core.threads()
            .create(
                "invalid1".into(),
                &normal.id,
                "Chat",
                Some("aylul"),
                Some("work")
            )
            .await
            .is_err()
    );
    assert!(
        core.threads()
            .create("invalid2".into(), &assistant.id, "Chat", None, None)
            .await
            .is_err()
    );
}

async fn core(w: &World) -> Arc<shadows_core::AppCore> {
    use shadows_core::testing::{LiveHandles, Runtime, Sessions, SessionsConfig};
    let (runtime, _) = Runtime::start(w.storage.clone()).await.unwrap();
    let root = w._tmp.path().join("home");
    std::fs::create_dir_all(&root).unwrap();
    let sessions = Sessions::new(
        Arc::new(shadows_agent::claude::ClaudeAdapter {
            node: std::env::current_exe().unwrap(),
            adapter: "unused.js".into(),
            agent: std::env::current_exe().unwrap(),
            adapter_version: "fixture".into(),
            agent_version: "fixture".into(),
        }),
        w.storage.clone(),
        SessionsConfig {
            secrets: Some(w.secrets.clone()),
            aylul: Some(shadows_core::AylulConfig {
                project_slug: "aylul".into(),
                home_root: root.clone(),
            }),
            ..Default::default()
        },
    );
    shadows_core::AppCore::assemble(shadows_core::CoreParts {
        storage: w.storage.clone(),
        secrets: w.secrets.clone(),
        runtime: Arc::new(runtime),
        sessions,
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(16).0,
        ui: tokio::sync::broadcast::channel(16).0,
        mcp_url: "".into(),
    })
}

#[tokio::test]
async fn aylul_thread_requires_a_live_agent() {
    let w = seeded().await;
    let core = core(&w).await;
    let p = core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &w._tmp.path().join("home"))
        .await
        .unwrap();
    assert!(
        core.threads()
            .create("t".into(), &p.id, "Chat", Some("aylul"), None)
            .await
            .is_err()
    );
    w.profiles
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    let t = core
        .threads()
        .create("t".into(), &p.id, "Chat", Some("aylul"), Some("work"))
        .await
        .unwrap();
    assert_eq!(t.profile.as_deref(), Some("work"));
    assert_eq!(
        w.storage
            .turn_context(&t.id)
            .await
            .unwrap()
            .profile
            .as_deref(),
        Some("work")
    );
    w.profiles.delete_agent("d".into(), "work").await.unwrap();
    assert!(
        core.threads()
            .create("t2".into(), &p.id, "Chat", Some("aylul"), Some("work"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn planner_thread_refuses_an_agent() {
    let w = seeded().await;
    let core = core(&w).await;
    let p = core
        .projects()
        .create(
            "p2".into(),
            "project",
            "Project",
            w._tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    assert!(
        core.threads()
            .create("t".into(), &p.id, "Chat", None, Some("work"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn threads_list_by_profile_only() {
    let w = seeded().await;
    let core = core(&w).await;
    let p = core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &w._tmp.path().join("home"))
        .await
        .unwrap();
    for name in ["Work", "Personal"] {
        w.profiles
            .create_agent(name.into(), agent(name))
            .await
            .unwrap();
    }
    let first = core
        .threads()
        .create("t1".into(), &p.id, "First", Some("aylul"), Some("work"))
        .await
        .unwrap();
    core.threads()
        .create(
            "t2".into(),
            &p.id,
            "Personal",
            Some("aylul"),
            Some("personal"),
        )
        .await
        .unwrap();
    let last = core
        .threads()
        .create("t3".into(), &p.id, "Last", Some("aylul"), Some("work"))
        .await
        .unwrap();
    let threads = core.threads().list_for_profile("work").await.unwrap();
    assert_eq!(
        threads.iter().map(|t| &t.id).collect::<Vec<_>>(),
        vec![&last.id, &first.id]
    );
}

#[tokio::test]
async fn legacy_aylul_threads_map_to_aylul() {
    let tmp = tempfile::tempdir().unwrap();
    let prior = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir(testing::migrations_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("0029_")
        {
            continue;
        }
        std::fs::copy(&path, prior.path().join(path.file_name().unwrap())).unwrap();
    }
    let db = tmp.path().join("legacy.db");
    let pool = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db)
            .create_if_missing(true),
    )
    .await
    .unwrap();
    sqlx::migrate::Migrator::new(prior.path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO project(id,slug,name,created_at,directory)
        VALUES('p','aylul','AYLUL','old',?)",
    )
    .bind(tmp.path().to_str().unwrap())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO planning_thread(id,project_id,title,status,created_at,harness_kind)
        VALUES('t','p','Legacy','Open','old','aylul')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let storage = testing::Storage::open(&db).await.unwrap();
    let t = storage
        .get_thread(&shadows_core::ThreadId::from_literal("t"))
        .await
        .unwrap();
    assert_eq!(t.profile.as_deref(), Some("aylul"));
}

#[tokio::test]
async fn fork_keeps_profile() {
    let w = seeded().await;
    let core = core(&w).await;
    let p = core
        .projects()
        .ensure_assistant("aylul", "AYLUL", &w._tmp.path().join("home"))
        .await
        .unwrap();
    w.profiles
        .create_agent("a".into(), agent("Work"))
        .await
        .unwrap();
    let t = core
        .threads()
        .create("t".into(), &p.id, "Chat", Some("aylul"), Some("work"))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO operation(id,kind,status_kind,thread_id,runtime_instance_id,
        created_at,started_at,finished_at,outcome_json) VALUES('op','PlannerTurn','Completed',?,
        (SELECT id FROM runtime_instance ORDER BY started_at DESC LIMIT 1),'now','now','now','{}')",
    )
    .bind(t.id.as_str())
    .execute(w.storage.reader())
    .await
    .unwrap();
    let entry = w
        .storage
        .append_thread_entry(
            &t.id,
            testing::NewThreadEntry {
                kind: shadows_core::ThreadEntryKind::AgentMessage,
                author: shadows_core::Actor::system(),
                body: "Completed",
                refs: &[],
                card: None,
                operation_id: Some(&shadows_core::OperationId::from_literal("op")),
            },
        )
        .await
        .unwrap();
    w.storage
        .record_harness_session(&t.id, "synthetic-session")
        .await
        .unwrap();
    let fork = core
        .threads()
        .fork("fork".into(), &t.id, &entry.id)
        .await
        .unwrap();
    assert_eq!(fork.profile.as_deref(), Some("work"));
    assert_eq!(
        w.storage
            .turn_context(&fork.id)
            .await
            .unwrap()
            .profile
            .as_deref(),
        Some("work")
    );
}

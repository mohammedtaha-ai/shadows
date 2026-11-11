//! The Planner makes a folder without git a repository: `git init` and one
//! empty commit in Shadows' name, refused inside a repository and to a grant
//! that is not a Planner's. Real git, no agent.
use super::*;

fn without_git(f: &Fixture) {
    std::fs::remove_dir_all(f.root.join(".git")).unwrap();
}

#[tokio::test]
async fn the_planner_initialises_git_in_a_folder_without_one() {
    let f = fixture().await;
    without_git(&f);
    let grant = planner_grant(&f).await;
    let commit = f.core.runs().init_git(&grant).await.unwrap();
    assert_eq!(commit.len(), 40, "{commit}");
    assert!(commit.chars().all(|c| c.is_ascii_hexdigit()), "{commit}");
    assert_eq!(git(&f.root, &["rev-parse", "HEAD"]).trim(), commit);
    assert_eq!(
        git(&f.root, &["log", "-1", "--format=%an"]).trim(),
        "Shadows"
    );
    assert_eq!(git(&f.root, &["ls-tree", "-r", "HEAD"]).trim(), "");
    let status = git(&f.root, &["status", "--porcelain"]);
    assert!(status.contains("?? README.md"), "{status}");
    let run = f.core.runs().task(&f.workflow, 1, 3).await.unwrap();
    assert!(run.start.allowed, "{:?}", run.start);
    f.finish().await;
}

#[tokio::test]
async fn git_init_is_refused_inside_a_repository() {
    let f = fixture().await;
    let grant = planner_grant(&f).await;
    let refused = f.core.runs().init_git(&grant).await;
    assert!(matches!(refused, Err(RunsError::AlreadyGit)), "{refused:?}");
    assert_eq!(git(&f.root, &["rev-list", "--count", "HEAD"]).trim(), "1");
    f.finish().await;
}

#[tokio::test]
async fn git_init_is_refused_to_an_executor_grant() {
    let f = fixture().await;
    without_git(&f);
    let thread: String = sqlx::query_scalar("SELECT written_by_thread FROM workflow WHERE id = ?")
        .bind(SOURCE)
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    let grant = shadows_core::Grant {
        id: shadows_core::GrantId::generate(),
        kind: shadows_core::GrantKind::Executor,
        project_id: f.project.clone(),
        thread_id: Some(ThreadId::from_literal(thread)),
        created_at: "2026-11-11T00:00:00Z".into(),
        revoked_at: None,
        scope: Some(shadows_core::testing::TaskScope {
            workflow: SOURCE.into(),
            version: 1,
            task: 3,
        }),
    };
    let refused = f.core.runs().init_git(&grant).await;
    assert!(matches!(refused, Err(RunsError::NotFound)), "{refused:?}");
    assert!(!f.root.join(".git").exists());
    f.finish().await;
}

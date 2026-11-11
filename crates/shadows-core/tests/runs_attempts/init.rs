//! The Planner makes a folder without git a repository: `git init` and a
//! first commit of the folder's files in Shadows' name, its secrets and
//! dependencies left out and untracked, refused inside a repository and to a
//! grant that is not a Planner's. Real git, no agent.
use super::*;

fn without_git(f: &Fixture) {
    std::fs::remove_dir_all(f.root.join(".git")).unwrap();
}

/// The paths HEAD's tree holds, in git's order.
fn tree(f: &Fixture) -> Vec<String> {
    git(&f.root, &["ls-tree", "-r", "--name-only", "HEAD"])
        .lines()
        .map(str::to_string)
        .collect()
}

fn write(f: &Fixture, path: &str, bytes: &[u8]) {
    let file = f.root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, bytes).unwrap();
}

#[tokio::test]
async fn git_init_takes_the_files_the_folder_holds() {
    let f = fixture().await;
    without_git(&f);
    write(&f, ".gitignore", b"notes.txt\n");
    write(&f, "notes.txt", b"mine");
    write(&f, "src/main.go", b"package main\n");
    write(&f, "assets/icon.ico", &[0, 0, 1, 0, 0, 255, 0]);
    let grant = planner_grant(&f).await;
    let commit = f.core.runs().init_git(&grant).await.unwrap();
    assert_eq!(git(&f.root, &["rev-parse", "HEAD"]).trim(), commit);
    assert_eq!(
        tree(&f),
        [".gitignore", "README.md", "assets/icon.ico", "src/main.go"]
    );
    assert!(f.root.join("notes.txt").exists());
    assert_eq!(
        git(&f.root, &["log", "-1", "--format=%an"]).trim(),
        "Shadows"
    );
    let run = f.core.runs().task(&f.workflow, 1, 3).await.unwrap();
    assert!(run.start.allowed, "{:?}", run.start);
    f.finish().await;
}

#[tokio::test]
async fn git_init_leaves_out_secrets_and_dependencies() {
    let f = fixture().await;
    without_git(&f);
    write(&f, ".env", b"KEY=x");
    write(&f, "node_modules/pkg/index.js", b"module.exports = 1;");
    write(&f, "src/main.go", b"package main\n");
    let grant = planner_grant(&f).await;
    let commit = f.core.runs().init_git(&grant).await.unwrap();
    assert_eq!(commit.len(), 40, "{commit}");
    assert_eq!(tree(&f), ["README.md", "src/main.go"]);
    assert_eq!(std::fs::read(f.root.join(".env")).unwrap(), b"KEY=x");
    assert_eq!(
        std::fs::read(f.root.join("node_modules/pkg/index.js")).unwrap(),
        b"module.exports = 1;"
    );
    let ignored = git(&f.root, &["status", "--porcelain", "--ignored"]);
    assert!(ignored.contains("!! .env"), "{ignored}");
    assert!(ignored.contains("!! node_modules/"), "{ignored}");
    assert_eq!(git(&f.root, &["status", "--porcelain"]).trim(), "");
    let exclude = std::fs::read_to_string(f.root.join(".git/info/exclude")).unwrap();
    let lines: Vec<&str> = exclude.lines().collect();
    assert!(lines.contains(&"/.env"), "{exclude}");
    assert!(lines.contains(&"/node_modules/"), "{exclude}");
    f.finish().await;
}

#[tokio::test]
async fn git_init_in_an_empty_folder_makes_an_empty_commit() {
    let f = fixture().await;
    without_git(&f);
    std::fs::remove_file(f.root.join("README.md")).unwrap();
    let grant = planner_grant(&f).await;
    let commit = f.core.runs().init_git(&grant).await.unwrap();
    assert_eq!(commit.len(), 40, "{commit}");
    assert_eq!(git(&f.root, &["ls-tree", "-r", "HEAD"]).trim(), "");
    f.finish().await;
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
    assert_eq!(tree(&f), ["README.md"]);
    assert_eq!(git(&f.root, &["status", "--porcelain"]).trim(), "");
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

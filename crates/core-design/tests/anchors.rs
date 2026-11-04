use core_db::{CommandContext, Storage, StorageError};
use core_design::{Design, DesignAnchor, DesignError, PartId};
use core_operations::Operations;
use core_projects::Projects;
use core_threads::{ThreadId, Threads};
use std::sync::Arc;

struct Fixture {
    _dir: tempfile::TempDir,
    design: Design,
    storage: Arc<Storage>,
    thread: ThreadId,
    part: PartId,
}

impl Fixture {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(Storage::open(&dir.path().join("test.db")).await.unwrap());
        let projects = Projects::new(storage.clone());
        let project = projects
            .create(
                "project".into(),
                "demo",
                "Demo",
                dir.path().to_str().unwrap(),
            )
            .await
            .unwrap();
        let thread = Threads::create_planning_thread_for_test(
            &storage,
            &CommandContext::user("thread".into(), "thread.create", serde_json::json!({})),
            &project.id,
            "Plan",
            "claude-code",
            None,
        )
        .await
        .unwrap();
        let design = Design::new(storage.clone(), Operations::new(storage.clone()));
        let part = design.seed_roadmap_for_tests(&project.id).await.unwrap();
        Self {
            _dir: dir,
            design,
            storage,
            thread: thread.id,
            part,
        }
    }
}

#[tokio::test]
async fn an_anchor_is_set_read_and_cleared() {
    let f = Fixture::new().await;
    let anchor = Some(DesignAnchor::Part(f.part));
    assert_eq!(f.design.thread_anchor(&f.thread).await.unwrap(), None);
    assert_eq!(
        f.design
            .set_thread_anchor("set".into(), &f.thread, anchor.clone())
            .await
            .unwrap(),
        anchor
    );
    assert_eq!(f.design.thread_anchor(&f.thread).await.unwrap(), anchor);
    assert_eq!(
        f.design
            .set_thread_anchor("clear".into(), &f.thread, None)
            .await
            .unwrap(),
        None
    );
    assert_eq!(f.design.thread_anchor(&f.thread).await.unwrap(), None);
}

#[tokio::test]
async fn an_anchor_from_another_project_is_refused() {
    let f = Fixture::new().await;
    let other = tempfile::tempdir().unwrap();
    let project = Projects::new(f.storage.clone())
        .create(
            "other".into(),
            "other",
            "Other",
            other.path().to_str().unwrap(),
        )
        .await
        .unwrap();
    let part = f.design.seed_roadmap_for_tests(&project.id).await.unwrap();
    let error = f
        .design
        .set_thread_anchor("set".into(), &f.thread, Some(DesignAnchor::Part(part)))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        DesignError::Storage(StorageError::NotFound(_))
    ));
    assert_eq!(f.design.thread_anchor(&f.thread).await.unwrap(), None);
}

#[tokio::test]
async fn a_replay_answers_the_first_anchor() {
    let f = Fixture::new().await;
    let anchor = Some(DesignAnchor::Part(f.part));
    f.design
        .set_thread_anchor("set".into(), &f.thread, anchor.clone())
        .await
        .unwrap();
    f.design
        .set_thread_anchor("clear".into(), &f.thread, None)
        .await
        .unwrap();
    assert_eq!(
        f.design
            .set_thread_anchor("set".into(), &f.thread, anchor.clone())
            .await
            .unwrap(),
        anchor
    );
    assert_eq!(f.design.thread_anchor(&f.thread).await.unwrap(), None);
    let error = f
        .design
        .set_thread_anchor("set".into(), &f.thread, None)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        DesignError::Storage(StorageError::CommandConflict)
    ));
}

#[tokio::test]
async fn an_aylul_thread_cannot_be_anchored() {
    let f = Fixture::new().await;
    let project = Threads::turn_context_for_test(&f.storage, &f.thread)
        .await
        .unwrap()
        .project_id;
    let thread = Threads::create_planning_thread_for_test(
        &f.storage,
        &CommandContext::user("aylul".into(), "thread.create", serde_json::json!({})),
        &project,
        "Assistant",
        "aylul",
        None,
    )
    .await
    .unwrap();
    let error = f
        .design
        .set_thread_anchor("set".into(), &thread.id, Some(DesignAnchor::Part(f.part)))
        .await
        .unwrap_err();
    assert!(matches!(error, DesignError::InvalidCommand(_)));
}

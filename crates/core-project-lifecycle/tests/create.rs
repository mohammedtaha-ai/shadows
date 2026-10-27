use core_code::{Code, CodeConfig};
use core_db::Storage;
use core_project_lifecycle::ProjectLifecycle;
use core_projects::Projects;
use std::sync::Arc;

#[tokio::test]
async fn creation_and_replay_activate_the_project_index() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("core.db")).await.unwrap());
    let projects = Projects::new(storage.clone());
    let code = Code::new(storage.clone(), projects.clone());
    let lifecycle = ProjectLifecycle::new(storage.clone(), projects, code.clone());
    code.start(CodeConfig::default()).await.unwrap();
    let directory = tmp.path().join("project");
    std::fs::create_dir(&directory).unwrap();
    let create = || {
        lifecycle.create(
            "register-project".into(),
            "project",
            "Project",
            directory.to_str().unwrap(),
        )
    };
    let project = create().await.unwrap();
    assert!(code.active_for_test().await.contains(&project.id));
    code.forget(&project.id).await;
    assert!(!code.active_for_test().await.contains(&project.id));
    let replay = create().await.unwrap();
    assert_eq!(replay.id, project.id);
    assert!(code.active_for_test().await.contains(&project.id));
    code.shut_down().await;

    storage.close().await.unwrap();
}

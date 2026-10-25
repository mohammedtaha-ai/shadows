use std::collections::BTreeMap;
use std::sync::Arc;

use core_db::Storage;
use core_projects::{Projects, ProjectsError};

#[tokio::test]
async fn public_registration_normalizes_paths_and_modes_before_replay() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let projects = Projects::new(storage);
    let raw = tmp.path().to_str().unwrap();
    let first = projects
        .create("create".into(), "demo", "Demo", raw)
        .await
        .unwrap();
    let replay = projects
        .create("create".into(), "demo", "Demo", raw)
        .await
        .unwrap();
    assert_eq!(first.id, replay.id);
    let modes = BTreeMap::from([(
        "claude-code".into(),
        vec!["acceptEdits".into(), "acceptEdits".into()],
    )]);
    projects
        .set_modes("modes".into(), &first.id, modes)
        .await
        .unwrap();
    let modes = BTreeMap::from([("claude-code".into(), vec!["acceptEdits".into()])]);
    let replay = projects
        .set_modes("modes".into(), &first.id, modes)
        .await
        .unwrap();
    assert_eq!(replay.allowed_modes["claude-code"], ["acceptEdits"]);
    let error = Projects::known_harness("unknown").unwrap_err();
    assert_eq!(error.to_string(), "harness unknown is not offered");
    assert!(matches!(error, ProjectsError::SettingNotOffered { .. }));
    assert!(matches!(
        projects
            .create("bad".into(), "bad", "Bad", "relative")
            .await,
        Err(ProjectsError::Directory(_))
    ));
    assert_eq!(projects.list().await.unwrap().len(), 1);
}

use super::*;
use shadows_core::{ContractHead, InitialFile, Stage};

pub(super) async fn setup() -> (
    tempfile::TempDir,
    Storage,
    Arc<AppCore>,
    ProjectId,
    std::path::PathBuf,
) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("project");
    std::fs::create_dir(&dir).unwrap();
    let db = tmp.path().join("s.sqlite3");
    let core = core_at(&db).await;
    let p = project(&core, "structure", &dir).await;
    core.design().seed_map_for_tests(&p).await.unwrap();
    let mut ops = super::stage::vision("Developers");
    for kind in ["backend", "database", "api", "frontend"] {
        ops.push(DesignOp::PartCreate {
            id: PartId::generate(),
            parent: None,
            before: None,
            content: PartContent {
                title: kind.into(),
                responsibility: format!("Own {kind}"),
                kind: Some(kind.into()),
                folders: vec![kind.into()],
                serves: vec!["G1".into()],
                files: vec![InitialFile {
                    path: format!("{kind}/src/initial.txt"),
                    content: format!("initial {kind}\n"),
                }],
                contract: ContractHead {
                    header: "Approved".into(),
                    rules: vec!["S1".into()],
                },
                ..Default::default()
            },
        });
    }
    core.plans()
        .edit_design("map".into(), &p, 0, ops)
        .await
        .unwrap();
    (tmp, Storage::open(&db).await.unwrap(), core, p, dir)
}

fn blocked<T: std::fmt::Debug, E: Into<CoreError>>(result: Result<T, E>, reason: &str) {
    match result.unwrap_err().into() {
        CoreError::Refused {
            code: shadows_core::ErrorCode::StageBlocked,
            message,
        } => {
            assert!(message.contains(reason), "{message}");
        }
        other => panic!("expected STAGE_BLOCKED {reason}: {other:?}"),
    }
}

#[tokio::test]
async fn create_structure_writes_exactly_the_approved_files() {
    let (_tmp, storage, core, p, dir) = setup().await;
    let result = core
        .design()
        .create_structure("write".into(), &p)
        .await
        .unwrap();
    assert_eq!(result.paths.len(), 9);
    assert_eq!(result.map_version, 2);
    assert!(!result.resumed);
    assert_eq!(core.design().stage(&p).await.unwrap().stage, Stage::Roadmap);
    core.design().seed_roadmap_for_tests(&p).await.unwrap();
    assert_eq!(core.design().stage(&p).await.unwrap().stage, Stage::Plans);
    for kind in ["backend", "database", "api", "frontend"] {
        assert_eq!(
            std::fs::read_to_string(dir.join(format!("{kind}/src/initial.txt"))).unwrap(),
            format!("initial {kind}\n")
        );
        let contract = std::fs::read_to_string(dir.join(format!("{kind}/contract.yaml"))).unwrap();
        let yaml = yaml_rust2::YamlLoader::load_from_str(&contract).unwrap();
        assert_eq!(yaml[0]["title"].as_str(), Some(kind));
        assert_eq!(yaml[0]["header"].as_str(), Some("Approved"));
    }
    assert!(
        std::fs::read_to_string(dir.join("SHADOWS.md"))
            .unwrap()
            .contains("map version 2")
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM durable_event WHERE kind='ProjectStructureWritten' AND project_id=?",
    )
    .bind(p.as_str())
    .fetch_one(storage.reader())
    .await
    .unwrap();
    assert_eq!(count, 1);

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_directory_holding_only_git_is_accepted() {
    for directory in [false, true] {
        let (_tmp, _storage, core, p, dir) = setup().await;
        if directory {
            std::fs::create_dir(dir.join(".git")).unwrap();
        } else {
            std::fs::write(dir.join(".git"), "gitdir: elsewhere").unwrap();
        }
        core.design()
            .create_structure("write".into(), &p)
            .await
            .unwrap();
        assert!(dir.join(".git").exists());

        core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
            .await
            .unwrap();
        _storage.close().await.unwrap();
    }
}

#[tokio::test]
async fn a_non_empty_directory_refuses_and_writes_nothing() {
    let (_tmp, storage, core, p, dir) = setup().await;
    std::fs::create_dir(dir.join("backend")).unwrap();
    blocked(
        core.design().create_structure("write".into(), &p).await,
        "directory not empty",
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM design_structure_write")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_missing_or_file_directory_refuses() {
    for file in [false, true] {
        let (_tmp, _storage, core, p, dir) = setup().await;
        std::fs::remove_dir(&dir).unwrap();
        if file {
            std::fs::write(&dir, "foreign").unwrap();
        }
        blocked(
            core.design().create_structure("write".into(), &p).await,
            "project directory missing",
        );

        core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
            .await
            .unwrap();
        _storage.close().await.unwrap();
    }
}

#[tokio::test]
async fn an_unapproved_map_or_missing_kind_refuses() {
    let (_tmp, storage, core, p, dir) = setup().await;
    sqlx::query("UPDATE design_workspace SET map_revision=0 WHERE project_id=?")
        .bind(p.as_str())
        .execute(storage.reader())
        .await
        .unwrap();
    blocked(
        core.design().create_structure("write".into(), &p).await,
        "map not approved",
    );
    sqlx::query("UPDATE design_workspace SET map_revision=2 WHERE project_id=?")
        .bind(p.as_str())
        .execute(storage.reader())
        .await
        .unwrap();
    core.standards()
        .save_standards_additions(
            "extra".into(),
            &p,
            shadows_core::StandardsAdditions {
                parts: vec![shadows_core::AdditionalPart {
                    name: "billing".into(),
                    owns: "Billing".into(),
                }],
                rules: vec![],
            },
        )
        .await
        .unwrap();
    blocked(
        core.design().create_structure("write".into(), &p).await,
        "missing part: billing",
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_second_call_refuses_and_a_replay_returns_the_result() {
    let (_tmp, _storage, core, p, _dir) = setup().await;
    let result = core
        .design()
        .create_structure("write".into(), &p)
        .await
        .unwrap();
    blocked(
        core.design().create_structure("second".into(), &p).await,
        "structure already written",
    );
    assert_eq!(
        core.design()
            .create_structure("write".into(), &p)
            .await
            .unwrap(),
        result
    );

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    _storage.close().await.unwrap();
}

#[tokio::test]
async fn a_stopped_write_resumes() {
    let (_tmp, _storage, core, p, dir) = setup().await;
    assert!(
        core.design()
            .create_structure_stopping_after("stop".into(), &p, 3)
            .await
            .is_err()
    );
    assert_eq!(
        core.design().stage(&p).await.unwrap().stage,
        Stage::Structure
    );
    assert!(dir.join("api/contract.yaml.shadows-tmp").exists());
    let result = core
        .design()
        .create_structure("resume".into(), &p)
        .await
        .unwrap();
    assert!(result.resumed);
    assert!(!dir.join("api/contract.yaml.shadows-tmp").exists());
    assert!(result.paths.iter().all(|path| dir.join(path).is_file()));
    assert_eq!(
        core.design()
            .create_structure("resume".into(), &p)
            .await
            .unwrap(),
        result
    );

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    _storage.close().await.unwrap();
}

#[tokio::test]
async fn a_foreign_or_changed_file_refuses_the_resume() {
    for foreign in [true, false] {
        let (_tmp, _storage, core, p, dir) = setup().await;
        assert!(
            core.design()
                .create_structure_stopping_after("stop".into(), &p, 1)
                .await
                .is_err()
        );
        let path = if foreign { "foreign.txt" } else { "SHADOWS.md" };
        std::fs::write(dir.join(path), "person's bytes").unwrap();
        blocked(
            core.design().create_structure("resume".into(), &p).await,
            path,
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(path)).unwrap(),
            "person's bytes"
        );
        assert!(dir.join("backend/contract.yaml.shadows-tmp").exists());
        assert!(!dir.join("backend/contract.yaml").exists());

        core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
            .await
            .unwrap();
        _storage.close().await.unwrap();
    }
}

#[tokio::test]
async fn a_reserved_path_from_an_older_map_refuses() {
    let (_tmp, storage, core, p, dir) = setup().await;
    let json: String = sqlx::query_scalar(
        "SELECT parts_json FROM design_map_version WHERE project_id=? AND number=2",
    )
    .bind(p.as_str())
    .fetch_one(storage.reader())
    .await
    .unwrap();
    let mut rows: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut content: serde_json::Value =
        serde_json::from_str(rows[0]["content_json"].as_str().unwrap()).unwrap();
    content["files"] = serde_json::json!([{"path":"backend/CONTRACT.yaml","content":"foreign"}]);
    rows[0]["content_json"] = serde_json::Value::String(content.to_string());
    sqlx::query("UPDATE design_map_version SET parts_json=? WHERE project_id=? AND number=2")
        .bind(rows.to_string())
        .bind(p.as_str())
        .execute(storage.reader())
        .await
        .unwrap();
    blocked(
        core.design().create_structure("write".into(), &p).await,
        "reserved path: backend/CONTRACT.yaml",
    );
    assert_eq!(std::fs::read_dir(dir).unwrap().count(), 0);

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn concurrent_calls_write_once() {
    let (_tmp, _storage, core, p, _dir) = setup().await;
    let (one, two) = tokio::join!(
        core.design().create_structure("one".into(), &p),
        core.design().create_structure("two".into(), &p)
    );
    assert_ne!(one.is_ok(), two.is_ok());
    blocked(
        if one.is_err() { one } else { two },
        "structure already written",
    );

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    _storage.close().await.unwrap();
}

#[tokio::test]
async fn a_recorded_file_with_a_temp_suffix_is_never_cleaned_up() {
    let (_tmp, _storage, core, p, dir) = setup().await;
    let backend = core
        .design()
        .parts(&p, None, None)
        .await
        .unwrap()
        .items
        .into_iter()
        .find(|part| part.content.title == "backend")
        .unwrap();
    let mut content = backend.content;
    for path in [
        "backend/contract.yaml.shadows-tmp",
        "backend/contract.yaml.shadows-tmp.shadows-tmp",
    ] {
        content.files.push(InitialFile {
            path: path.into(),
            content: "approved starter".into(),
        });
    }
    core.plans()
        .edit_design(
            "temp-suffix".into(),
            &p,
            1,
            vec![DesignOp::PartPut {
                id: backend.id,
                content,
            }],
        )
        .await
        .unwrap();
    assert!(
        core.design()
            .create_structure_stopping_after("stop".into(), &p, 8)
            .await
            .is_err()
    );
    let recorded = dir.join("backend/contract.yaml.shadows-tmp");
    std::fs::write(&recorded, "person's changed bytes").unwrap();
    blocked(
        core.design().create_structure("resume".into(), &p).await,
        "backend/contract.yaml.shadows-tmp",
    );
    assert_eq!(
        std::fs::read_to_string(&recorded).unwrap(),
        "person's changed bytes"
    );
    assert!(dir.join("database/src/initial.txt.shadows-tmp").exists());
    std::fs::write(&recorded, "approved starter").unwrap();
    let written = core
        .design()
        .create_structure("resume".into(), &p)
        .await
        .unwrap();
    assert!(written.resumed);
    for path in [
        "backend/contract.yaml.shadows-tmp",
        "backend/contract.yaml.shadows-tmp.shadows-tmp",
    ] {
        assert_eq!(
            std::fs::read_to_string(dir.join(path)).unwrap(),
            "approved starter"
        );
    }

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    _storage.close().await.unwrap();
}

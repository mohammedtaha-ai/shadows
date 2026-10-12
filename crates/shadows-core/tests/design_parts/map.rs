use super::*;

#[path = "map_proposals.rs"]
mod proposals;

async fn setup() -> (tempfile::TempDir, Storage, Arc<AppCore>, ProjectId) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("map.sqlite3");
    let core = core_at(&path).await;
    let p = project(&core, "map-tests", tmp.path()).await;
    let storage = Storage::open(&path).await.unwrap();
    storage.seed_map_for_tests(&p).await.unwrap();
    (tmp, storage, core, p)
}

fn mapped(id: &PartId, parent: Option<&PartId>, title: &str, folders: &[&str]) -> DesignOp {
    let mut c = content(title);
    c.folders = folders.iter().map(|s| s.to_string()).collect();
    DesignOp::PartCreate {
        id: id.clone(),
        parent: parent.cloned(),
        before: None,
        content: c,
    }
}

#[tokio::test]
async fn folders_nest_only_under_their_parent() {
    let (_tmp, _storage, core, p) = setup().await;
    let (parent, child) = (PartId::generate(), PartId::generate());
    edit(
        &core,
        &p,
        0,
        vec![
            mapped(&parent, None, "backend", &["backend"]),
            mapped(&child, Some(&parent), "core", &["backend/src/core"]),
        ],
    )
    .await;
    let result = core
        .design()
        .edit(
            "overlap".into(),
            &p,
            1,
            vec![mapped(
                &PartId::generate(),
                None,
                "sibling",
                &["BACKEND/src"],
            )],
        )
        .await;
    assert!(result.is_err());
    assert_eq!(core.design().vision(&p).await.unwrap().revision, 1);
}

#[tokio::test]
async fn a_folder_with_dot_dot_or_absolute_is_refused() {
    let (_tmp, _storage, core, p) = setup().await;
    for path in ["../x", "/x", "C:/x", "a//b", "a/./b", "a/x."] {
        assert!(
            core.design()
                .edit(
                    format!("bad-{path}"),
                    &p,
                    0,
                    vec![mapped(&PartId::generate(), None, "x", &[path])]
                )
                .await
                .is_err()
        );
    }
    let id = PartId::generate();
    edit(&core, &p, 0, vec![mapped(&id, None, "x", &["./a/b/"])]).await;
    assert_eq!(
        core.design()
            .part(&p, &id)
            .await
            .unwrap()
            .part
            .content
            .folders,
        ["a/b"]
    );
}

#[tokio::test]
async fn a_hand_edit_cannot_drop_a_mandatory_part() {
    let (_tmp, _storage, core, p) = setup().await;
    let backend = PartId::generate();
    let api = PartId::generate();
    let mut c = content("backend");
    c.kind = Some("backend".into());
    edit(
        &core,
        &p,
        0,
        vec![
            DesignOp::PartCreate {
                id: backend.clone(),
                parent: None,
                before: None,
                content: c,
            },
            mapped(&api, None, "api", &[]),
        ],
    )
    .await;
    let mut c = core.design().part(&p, &backend).await.unwrap().part.content;
    c.kind = None;
    for (n, op) in [
        DesignOp::PartPut {
            id: backend.clone(),
            content: c,
        },
        DesignOp::PartMove {
            id: backend,
            parent: Some(api),
            before: None,
        },
    ]
    .into_iter()
    .enumerate()
    {
        assert!(matches!(
            core.design()
                .edit(format!("drop-{n}"), &p, 1, vec![op])
                .await,
            Err(CoreError::Storage(StorageError::StageBlocked(_)))
        ));
    }
    assert_eq!(core.design().vision(&p).await.unwrap().revision, 1);
}

#[tokio::test]
async fn removed_links_and_metadata_edits_preserve_drift() {
    let (_tmp, _storage, core, p) = setup().await;
    let id = PartId::generate();
    let mut c = content("core");
    c.serves = vec!["G1".into()];
    edit(
        &core,
        &p,
        0,
        vec![
            DesignOp::VisionItemAdd {
                field: shadows_core::VisionField::Goals,
                text: "original goal".into(),
            },
            DesignOp::PartCreate {
                id: id.clone(),
                parent: None,
                before: None,
                content: c,
            },
        ],
    )
    .await;
    edit(
        &core,
        &p,
        1,
        vec![DesignOp::VisionItemPut {
            id: "G1".into(),
            text: "changed".into(),
        }],
    )
    .await;
    let mut c = core.design().part(&p, &id).await.unwrap().part.content;
    c.title = "renamed".into();
    edit(
        &core,
        &p,
        2,
        vec![DesignOp::PartPut {
            id: id.clone(),
            content: c,
        }],
    )
    .await;
    assert!(
        core.design()
            .stage(&p)
            .await
            .unwrap()
            .drift
            .iter()
            .any(|d| d.contains("G1 changed"))
    );
    edit(
        &core,
        &p,
        3,
        vec![DesignOp::VisionItemRemove { id: "G1".into() }],
    )
    .await;
    edit(
        &core,
        &p,
        4,
        vec![mapped(&PartId::generate(), None, "unrelated", &["web"])],
    )
    .await;
    assert!(
        core.design()
            .stage(&p)
            .await
            .unwrap()
            .drift
            .iter()
            .any(|d| d.contains("was removed"))
    );
}

#[tokio::test]
async fn initial_files_cannot_escape_part_ownership() {
    let (_tmp, _storage, core, p) = setup().await;
    let mut c = content("core");
    c.folders = vec!["src".into()];
    c.files = vec![shadows_core::InitialFile {
        path: "other/x.rs".into(),
        content: String::new(),
    }];
    assert!(
        core.design()
            .edit(
                "file".into(),
                &p,
                0,
                vec![DesignOp::PartCreate {
                    id: PartId::generate(),
                    parent: None,
                    before: None,
                    content: c
                }]
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn initial_files_cannot_also_be_directories() {
    let (_tmp, _storage, core, p) = setup().await;
    for (n, (folder, files)) in [
        ("SRC/config", vec!["src/CONFIG"]),
        ("src/config/nested", vec!["src/config"]),
        ("src", vec!["src/config", "SRC/CONFIG/settings.toml"]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut c = content("core");
        c.folders = vec!["src".into()];
        c.files = files
            .into_iter()
            .map(|path| shadows_core::InitialFile {
                path: path.into(),
                content: String::new(),
            })
            .collect();
        let parent = PartId::generate();
        let mut ops = vec![DesignOp::PartCreate {
            id: parent.clone(),
            parent: None,
            before: None,
            content: c,
        }];
        if folder != "src" {
            ops.push(mapped(
                &PartId::generate(),
                Some(&parent),
                "child",
                &[folder],
            ));
        }
        let result = core
            .design()
            .edit(format!("collision-{n}"), &p, 0, ops)
            .await;
        assert!(
            result.is_err(),
            "file/directory collision was accepted: {folder}"
        );
        assert_eq!(core.design().vision(&p).await.unwrap().revision, 0);
        assert!(
            core.design()
                .parts(&p, None, None)
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
}

#[tokio::test]
async fn a_hand_part_create_before_the_first_map_is_stage_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("map.sqlite3")).await;
    let p = project(&core, "map-gate", tmp.path()).await;
    let id = PartId::generate();
    let result = core
        .design()
        .edit("blocked".into(), &p, 0, vec![create(&id, None, "core")])
        .await;
    assert!(matches!(
        result,
        Err(CoreError::Storage(StorageError::StageBlocked(_)))
    ));
    assert_eq!(core.design().vision(&p).await.unwrap().revision, 0);
    let storage = Storage::open(&tmp.path().join("map.sqlite3"))
        .await
        .unwrap();
    storage.seed_map_for_tests(&p).await.unwrap();
    edit(&core, &p, 0, vec![create(&id, None, "core")]).await;
    let versions: i64 = sqlx::query_scalar("SELECT count(*) FROM design_map_version")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(versions, 2);
}

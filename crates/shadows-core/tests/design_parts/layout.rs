use super::*;

#[tokio::test]
async fn layout_lists_the_rules_for_each_kind() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("test.sqlite3")).await;
    let p = project(&core, "layout", tmp.path()).await;
    let backend = PartId::generate();
    let api = PartId::generate();
    let make = |id: &PartId, kind: &str| DesignOp::PartCreate {
        id: id.clone(),
        parent: None,
        before: None,
        content: PartContent {
            title: kind.into(),
            kind: Some(kind.into()),
            folder: kind.into(),
            layers: catalog_layers(kind, kind),
            ..Default::default()
        },
    };
    edit(
        &core,
        &p,
        0,
        vec![make(&backend, "backend"), make(&api, "api")],
    )
    .await;
    let layout = core.design().layout(&p).await.unwrap();
    let backend = layout.parts.iter().find(|part| part.id == backend).unwrap();
    let api = layout.parts.iter().find(|part| part.id == api).unwrap();
    assert!(backend.rules.iter().any(|rule| rule.id == "S1"));
    assert!(backend.rules.iter().any(|rule| rule.id == "S10"));
    assert!(!api.rules.iter().any(|rule| rule.id == "S1"));
}

#[tokio::test]
async fn layout_joins_layers_and_owns_nested_and_external_places() {
    use shadows_core::OwnedVia;
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("test.sqlite3")).await;
    let p = project(&core, "layout-paths", tmp.path()).await;
    let backend = PartId::generate();
    let db = PartId::generate();
    let mut layers = catalog_layers("backend", "backend");
    layers[0].place = "backend/app/Modules/{module}/Domain".into();
    let mut database = catalog_layers("database", "backend/database");
    database[2].place = "backend/config/database.php".into();
    edit(
        &core,
        &p,
        0,
        vec![
            DesignOp::PartCreate {
                id: backend.clone(),
                parent: None,
                before: None,
                content: PartContent {
                    title: "Backend".into(),
                    kind: Some("backend".into()),
                    folder: "backend".into(),
                    layers,
                    ..Default::default()
                },
            },
            DesignOp::PartCreate {
                id: db.clone(),
                parent: Some(backend.clone()),
                before: None,
                content: PartContent {
                    title: "Database".into(),
                    kind: Some("database".into()),
                    folder: "backend/database".into(),
                    layers: database,
                    ..Default::default()
                },
            },
        ],
    )
    .await;
    let layout = core.design().layout(&p).await.unwrap();
    let backend_part = layout.parts.iter().find(|part| part.id == backend).unwrap();
    assert!(!backend_part.layers[0].role.is_empty());
    assert!(backend_part.layers[0].may_use.is_empty());
    let owners = layout.owners("BACKEND/config/database.php");
    assert_eq!(owners.len(), 2);
    assert_eq!(owners[0].part, backend);
    assert_eq!(owners[0].via, OwnedVia::Folder);
    assert_eq!(owners[1].part, db);
    assert_eq!(owners[1].via, OwnedVia::Layer("connection".into()));
    let owners = layout.owners("backend/database/migrations/x.php");
    assert_eq!(owners[0].part, db);
    assert!(!owners.iter().any(|o| o.part == backend));
    assert!(
        layout
            .owners("backend/app/Modules/Tickets/Domain/Ticket.php")
            .iter()
            .any(|o| o.via == OwnedVia::Layer("domain".into()))
    );
    assert!(
        !layout
            .owners("backend/app/Modules/Tickets/Http/Ticket.php")
            .iter()
            .any(|o| o.via == OwnedVia::Layer("domain".into()))
    );
    for path in [
        "backend2/x",
        "/backend/x",
        "backend/../x",
        "backend\\x",
        "C:/backend/x",
    ] {
        assert!(layout.owners(path).is_empty(), "{path}");
    }
    assert_eq!(layout.owners("./backend//config/database.php/").len(), 2);
    let mut layout = layout;
    layout
        .parts
        .iter_mut()
        .find(|p| p.id == backend)
        .unwrap()
        .layers[0]
        .place = Some("backend/core-{module}-{module}/Domain".into());
    assert!(
        layout
            .owners("backend/core-a-b/Domain/x")
            .iter()
            .any(|o| o.via == OwnedVia::Layer("domain".into()))
    );
    assert!(
        !layout
            .owners("backend/core--/Domain/x")
            .iter()
            .any(|o| o.via == OwnedVia::Layer("domain".into()))
    );
}

#[tokio::test]
async fn layout_snapshot_keeps_parts_and_standards_consistent() {
    use shadows_core::{Design, StandardRule, StandardsAdditions};
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.sqlite3");
    let core = core_at(&db).await;
    let p = project(&core, "snapshot", tmp.path()).await;
    edit(
        &core,
        &p,
        0,
        vec![create(&PartId::generate(), None, "Part")],
    )
    .await;
    let storage = Storage::open(&db).await.unwrap();
    let mut tx = storage.reader().begin().await.unwrap();
    let before = Design::layout_in(&mut tx, &p).await.unwrap();
    core.standards()
        .save_standards_additions(
            "new-rules".into(),
            &p,
            StandardsAdditions {
                rules: vec![StandardRule {
                    id: "P1".into(),
                    text: "Snapshot rule".into(),
                    parts: vec![],
                }],
                parts: vec![],
            },
        )
        .await
        .unwrap();
    let during = Design::layout_in(&mut tx, &p).await.unwrap();
    assert_eq!(before.standards_additions, during.standards_additions);
    assert!(!during.parts[0].rules.iter().any(|r| r.id == "P1"));
    tx.commit().await.unwrap();
    let after = core.design().layout(&p).await.unwrap();
    assert_eq!(after.standards_additions, Some(1));
    assert!(after.parts[0].rules.iter().any(|r| r.id == "P1"));
    core.project_lifecycle()
        .remove("remove".into(), &p)
        .await
        .unwrap();
    assert!(core.design().layout(&p).await.is_err());
    assert!(core.design().layout(&ProjectId::generate()).await.is_err());
    storage.close().await.unwrap();
}

#[tokio::test]
async fn layout_marks_grouping_parts_without_copying_the_rule() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("test.sqlite3")).await;
    let p = project(&core, "group", tmp.path()).await;
    let parent = PartId::generate();
    let child = PartId::generate();
    edit(
        &core,
        &p,
        0,
        vec![
            DesignOp::PartCreate {
                id: parent.clone(),
                parent: None,
                before: None,
                content: PartContent {
                    title: "Clients".into(),
                    kind: Some("frontend".into()),
                    folder: "clients".into(),
                    ..Default::default()
                },
            },
            DesignOp::PartCreate {
                id: child.clone(),
                parent: Some(parent.clone()),
                before: None,
                content: PartContent {
                    title: "Web".into(),
                    kind: Some("frontend".into()),
                    folder: "clients/web".into(),
                    layers: catalog_layers("frontend", "clients/web"),
                    ..Default::default()
                },
            },
        ],
    )
    .await;
    let layout = core.design().layout(&p).await.unwrap();
    assert!(
        layout
            .parts
            .iter()
            .find(|p| p.id == parent)
            .unwrap()
            .grouping
    );
    assert!(
        !layout
            .parts
            .iter()
            .find(|p| p.id == child)
            .unwrap()
            .grouping
    );
}

use shadows_core::testing::{LiveHandles, Storage, acp};
use shadows_core::{
    AcceptanceItem, AppCore, CoreError, CoreParts, DesignAnchor, DesignOp, DraftStart, PartContent,
    PartId, PlanEdit, PlanOp, ProjectId, StorageError, TaskContent,
};
use std::sync::Arc;

#[path = "design_parts/stage.rs"]
mod stage;

#[path = "design_parts/map.rs"]
mod map;
#[path = "design_parts/roadmap.rs"]
mod roadmap;
#[path = "design_parts/roadmap_changes.rs"]
mod roadmap_changes;
#[path = "design_parts/roadmap_gate.rs"]
mod roadmap_gate;
#[path = "design_parts/structure.rs"]
mod structure;

async fn core_at(path: &std::path::Path) -> Arc<AppCore> {
    let storage = Arc::new(Storage::open(path).await.unwrap());
    let runtime = Arc::new(
        shadows_core::AppCore::runtime_for_tests(storage.clone())
            .await
            .unwrap()
            .0,
    );
    AppCore::assemble(CoreParts {
        operations: shadows_core::AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime,
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(32).0,
        ui: tokio::sync::broadcast::channel(32).0,
        mcp_url: acp::MCP_URL.into(),
    })
}

async fn project(core: &AppCore, slug: &str, dir: &std::path::Path) -> ProjectId {
    core.project_lifecycle()
        .create(format!("project-{slug}"), slug, slug, dir.to_str().unwrap())
        .await
        .unwrap()
        .id
}

fn content(title: &str) -> PartContent {
    PartContent {
        title: title.into(),
        responsibility: " مسؤولية ".into(),
        design: " تصميم\n".into(),
        kind: Some("طبقة".into()),
        ..Default::default()
    }
}

fn create(id: &PartId, parent: Option<&PartId>, title: &str) -> DesignOp {
    DesignOp::PartCreate {
        id: id.clone(),
        parent: parent.cloned(),
        before: None,
        content: content(title),
    }
}

async fn edit(core: &AppCore, p: &ProjectId, rev: i64, ops: Vec<DesignOp>) {
    if ops
        .iter()
        .any(|op| matches!(op, DesignOp::PartCreate { .. }))
    {
        core.design().seed_map_for_tests(p).await.unwrap();
    }
    assert_eq!(
        core.plans()
            .edit_design(format!("edit-{p}-{rev}"), p, rev, ops)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .revision,
        rev + 1
    );
}

// Missing cycle/ownership checks or destructive reparenting must fail this test.
#[tokio::test]
async fn part_moves_keep_identity_and_reject_cycles() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("test.sqlite3")).await;
    let p = project(&core, "one", tmp.path()).await;
    let other_dir = tmp.path().join("other");
    std::fs::create_dir(&other_dir).unwrap();
    let q = project(&core, "two", &other_dir).await;
    let (a, b, c, foreign) = (
        PartId::generate(),
        PartId::generate(),
        PartId::generate(),
        PartId::generate(),
    );
    edit(
        &core,
        &p,
        0,
        vec![
            create(&a, None, " أ "),
            create(&b, Some(&a), "ب"),
            create(&c, Some(&b), "ج"),
        ],
    )
    .await;
    edit(&core, &q, 0, vec![create(&foreign, None, "آخر")]).await;
    let events = || core.design().vision(&p);
    assert_eq!(events().await.unwrap().revision, 1);
    for parent in [&c, &foreign, &a] {
        let result = core
            .plans()
            .edit_design(
                format!("bad-{parent}"),
                &p,
                1,
                vec![
                    DesignOp::PartPut {
                        id: b.clone(),
                        content: content("must roll back"),
                    },
                    DesignOp::PartMove {
                        id: a.clone(),
                        parent: Some(parent.clone()),
                        before: None,
                    },
                ],
            )
            .await
            .map_err(shadows_core::CoreError::from);
        assert!(result.is_err());
        assert_eq!(
            core.plans()
                .design_part(&p, &b)
                .await
                .map_err(shadows_core::CoreError::from)
                .unwrap()
                .part
                .content
                .title,
            "ب"
        );
        assert_eq!(core.design().vision(&p).await.unwrap().revision, 1);
    }
    let move_b = vec![DesignOp::PartMove {
        id: b.clone(),
        parent: None,
        before: Some(a.clone()),
    }];
    let (x, y) = tokio::join!(
        core.plans()
            .edit_design("move-one".into(), &p, 1, move_b.clone()),
        core.plans().edit_design("move-two".into(), &p, 1, move_b)
    );
    assert_eq!(usize::from(x.is_ok()) + usize::from(y.is_ok()), 1);
    let failed = if x.is_err() { x } else { y };
    assert!(matches!(
        failed.map_err(CoreError::from),
        Err(CoreError::Storage(StorageError::RevisionConflict {
            current: 2,
            ..
        }))
    ));
    edit(
        &core,
        &p,
        2,
        vec![DesignOp::PartPut {
            id: b.clone(),
            content: content(" جديد "),
        }],
    )
    .await;
    let v = core
        .plans()
        .design_part(&p, &c)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(
        v.ancestors.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
        vec![b.clone()]
    );
    assert_eq!(v.ancestors[0].content.title, "جديد");
    assert_eq!(v.part.id, c);
    assert_eq!(v.ancestors[0].content.design, " تصميم\n");
    assert!(
        core.plans()
            .design_part(&q, &b)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    // A failed batch journals nothing; successes have one event each.
    let conn = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new().filename(tmp.path().join("test.sqlite3")),
    )
    .await
    .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM durable_event WHERE kind='ProjectDesignChanged' AND project_id=?",
    )
    .bind(p.as_str())
    .fetch_one(&conn)
    .await
    .unwrap();
    assert_eq!(count, 3);
}

// Lost ordinal writes, invalid cursors or plan-content mutation must fail.
#[tokio::test]
async fn part_order_is_stable_across_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.sqlite3");
    let core = core_at(&db).await;
    let p = project(&core, "one", tmp.path()).await;
    let (a, b, c) = (PartId::generate(), PartId::generate(), PartId::generate());
    edit(
        &core,
        &p,
        0,
        vec![
            create(&a, None, "A"),
            create(&b, None, "B"),
            DesignOp::PartCreate {
                id: c.clone(),
                parent: None,
                before: Some(b.clone()),
                content: content("C"),
            },
        ],
    )
    .await;
    let seeded = core.design().seed_roadmap_for_tests(&p).await.unwrap();
    let grant = core.grants().issue("grant".into(), &p).await.unwrap().grant;
    let draft_ref = core
        .plans()
        .prepare_draft(&grant)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let start = core
        .plans()
        .start_draft(
            &grant,
            DraftStart {
                title: Some("Plan".into()),
                goal: Some("Goal".into()),
                plan_id: None,
                reason: None,
                draft_ref: Some(draft_ref),
                parts: vec![seeded],
            },
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    core.plans()
        .edit(
            &grant,
            PlanEdit {
                workflow_id: Some(start.workflow_id.clone()),
                expected_revision: 0,
                command_id: Some("plan-edit".into()),
                ops: vec![PlanOp::TaskAdd {
                    task: TaskContent {
                        number: 1,
                        title: "Task".into(),
                        goal: "Goal".into(),
                        reads: vec![],
                        writes: vec!["file.rs".into()],
                        acceptance: vec![AcceptanceItem {
                            number: 1,
                            text: "works".into(),
                        }],
                    },
                }],
            },
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    core.plans()
        .approve("approve".into(), &start.workflow_id, 1)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    core.plans()
        .archive("archive".into(), &start.plan_id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let frozen = core
        .plans()
        .get(&start.workflow_id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap()
        .content();
    let link = DesignOp::PlanLinkPut {
        anchor: DesignAnchor::Part(a.clone()),
        plan: start.plan_id.clone(),
    };
    edit(
        &core,
        &p,
        1,
        vec![
            link.clone(),
            link,
            DesignOp::PartMove {
                id: a.clone(),
                parent: None,
                before: None,
            },
        ],
    )
    .await;
    assert_eq!(
        core.plans()
            .design_part(&p, &a)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .plans,
        vec![start.plan_id.clone()]
    );
    assert_eq!(
        core.design()
            .parts(&p, None, None)
            .await
            .unwrap()
            .items
            .iter()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>(),
        vec![c.clone(), b.clone(), a.clone()]
    );
    for bad in [
        DesignOp::PartMove {
            id: b.clone(),
            parent: None,
            before: Some(b.clone()),
        },
        DesignOp::PartMove {
            id: b.clone(),
            parent: Some(a.clone()),
            before: Some(c.clone()),
        },
    ] {
        assert!(
            core.plans()
                .edit_design(uuid::Uuid::new_v4().to_string(), &p, 2, vec![bad])
                .await
                .map_err(shadows_core::CoreError::from)
                .is_err()
        );
    }
    assert!(core.design().parts(&p, Some(&a), Some(&c)).await.is_err());
    let qdir = tmp.path().join("q");
    std::fs::create_dir(&qdir).unwrap();
    let q = project(&core, "two", &qdir).await;
    let foreign = PartId::generate();
    edit(&core, &q, 0, vec![create(&foreign, None, "Foreign")]).await;
    assert!(
        core.plans()
            .edit_design(
                "foreign-link".into(),
                &q,
                1,
                vec![DesignOp::PlanLinkPut {
                    anchor: DesignAnchor::Part(foreign),
                    plan: start.plan_id.clone()
                }]
            )
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    assert_eq!(
        core.plans()
            .get(&start.workflow_id)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .content(),
        frozen
    );
    core.code().shut_down_for_test().await;
    drop(core);
    let reopened = core_at(&db).await;
    let page = reopened.design().parts(&p, None, None).await.unwrap();
    assert_eq!(page.revision, 2);
    assert_eq!(
        page.items.iter().map(|p| p.id.clone()).collect::<Vec<_>>(),
        vec![c, b.clone(), a.clone()]
    );
    assert_eq!(
        reopened
            .plans()
            .design_part(&p, &a)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .plans,
        vec![start.plan_id.clone()]
    );
    edit(
        &reopened,
        &p,
        2,
        vec![
            DesignOp::PartPut {
                id: a.clone(),
                content: content("Renamed"),
            },
            DesignOp::PartMove {
                id: a.clone(),
                parent: Some(b.clone()),
                before: None,
            },
        ],
    )
    .await;
    let moved = reopened
        .plans()
        .design_part(&p, &a)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(moved.plans, vec![start.plan_id.clone()]);
    assert_eq!(moved.ancestors[0].id, b);
    edit(
        &reopened,
        &p,
        3,
        vec![DesignOp::PlanLinkRemove {
            anchor: DesignAnchor::Part(a.clone()),
            plan: start.plan_id,
        }],
    )
    .await;
    assert!(
        reopened
            .plans()
            .design_part(&p, &a)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .plans
            .is_empty()
    );
    assert_eq!(
        reopened
            .plans()
            .get(&start.workflow_id)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .content(),
        frozen
    );
}

// Unbounded reads, missing page boundary or failed replay must fail.
#[tokio::test]
async fn part_pages_are_bounded_and_replay_is_immutable() {
    let tmp = tempfile::tempdir().unwrap();
    let core = core_at(&tmp.path().join("test.sqlite3")).await;
    let p = project(&core, "one", tmp.path()).await;
    let ids: Vec<_> = (0..51).map(|_| PartId::generate()).collect();
    core.design().seed_map_for_tests(&p).await.unwrap();
    let ops = ids
        .iter()
        .enumerate()
        .map(|(i, id)| create(id, None, &format!("قسم {i}")))
        .collect::<Vec<_>>();
    core.plans()
        .edit_design("many".into(), &p, 0, ops.clone())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let first = core.design().parts(&p, None, None).await.unwrap();
    assert_eq!(first.items.len(), 50);
    assert_eq!(first.next, Some(ids[49].clone()));
    let last = core
        .design()
        .parts(&p, None, first.next.as_ref())
        .await
        .unwrap();
    assert_eq!(last.items.len(), 1);
    assert_eq!(last.items[0].id, ids[50]);
    assert!(last.next.is_none());
    edit(
        &core,
        &p,
        1,
        vec![DesignOp::PartPut {
            id: ids[0].clone(),
            content: content("Renamed"),
        }],
    )
    .await;
    assert_eq!(
        core.plans()
            .edit_design("many".into(), &p, 0, ops)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .revision,
        1
    );
    core.project_lifecycle()
        .remove("remove".into(), &p)
        .await
        .unwrap();
    assert!(core.design().parts(&p, None, None).await.is_err());
    assert!(
        core.plans()
            .design_part(&p, &ids[0])
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    assert!(
        core.plans()
            .edit_design(
                "after-remove".into(),
                &p,
                2,
                vec![create(&PartId::generate(), None, "no")]
            )
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    // Replay remains valid after project removal; no new write occurs.
    assert_eq!(
        core.plans()
            .edit_design(
                format!("edit-{p}-1"),
                &p,
                1,
                vec![DesignOp::PartPut {
                    id: ids[0].clone(),
                    content: content("Renamed")
                }]
            )
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .revision,
        2
    );
}

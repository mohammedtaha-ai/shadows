#[allow(dead_code)]
#[path = "fixtures/cross_plan.rs"]
mod cross_plan;
use cross_plan::{SOURCE, command, fixture, task};
use shadows_core::{AppCore, Grant, Grants, OwnedVia, ProjectId, ThreadId, WorkflowId, Writer};

async fn grant(storage: &std::sync::Arc<shadows_core::testing::Storage>) -> Grant {
    Grants::issue_thread_grant_for_test(storage, &ThreadId::from_literal("writer"))
        .await
        .unwrap()
        .0
}

#[tokio::test]
async fn task_context_refuses_drafts_then_returns_frozen_content_without_writes() {
    let (_tmp, storage) = fixture().await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let grant = grant(&storage).await;
    let workflow = WorkflowId::from_literal(SOURCE);
    plans
        .edit_plan_for_test(
            &command("task"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[task(1)],
        )
        .await
        .unwrap();
    assert!(
        plans
            .task_context(&grant, Some(&workflow), 1)
            .await
            .unwrap_err()
            .to_string()
            .contains("approved")
    );
    plans
        .approve_plan_for_test(&command("approve"), &workflow, 1)
        .await
        .unwrap();
    let before: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM command_record), (SELECT count(*) FROM durable_event)",
    )
    .fetch_one(storage.reader())
    .await
    .unwrap();
    let context = plans
        .task_context(&grant, Some(&workflow), 1)
        .await
        .unwrap();
    assert_eq!(context.plan.workflow_id, workflow);
    assert_eq!(context.plan.version, 1);
    assert_eq!(context.task.title, "Task 1");
    assert_eq!(context.unowned_paths, vec!["src/result.rs"]);
    assert!(
        !context.parts.is_empty(),
        "tied parts without paths are retained"
    );
    assert!(context.parts.iter().all(|p| p.paths.is_empty()));
    let after: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM command_record), (SELECT count(*) FROM durable_event)",
    )
    .fetch_one(storage.reader())
    .await
    .unwrap();
    assert_eq!(before, after);
    sqlx::query(
        "INSERT INTO workflow (id,plan_id,state,version,revision,title,goal,
         written_by_thread,previous_version_id,created_at,updated_at)
         SELECT 'new-draft',plan_id,'Draft',2,0,'Changed','Changed',
         written_by_thread,id,created_at,updated_at FROM workflow WHERE id=?",
    )
    .bind(workflow.as_str())
    .execute(storage.reader())
    .await
    .unwrap();
    assert_eq!(
        plans
            .task_context(&grant, Some(&workflow), 1)
            .await
            .unwrap()
            .task
            .title,
        "Task 1"
    );

    let err = plans
        .task_context(&grant, Some(&workflow), 99)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("T99"));
    assert!(plans.task_context(&grant, None, 1).await.is_err());
    let mut foreign = grant.clone();
    foreign.project_id = ProjectId::from_literal("foreign");
    assert!(
        plans
            .task_context(&foreign, Some(&workflow), 1)
            .await
            .is_err()
    );
    storage.close().await.unwrap();
}

#[tokio::test]
async fn task_context_joins_layout_rules_nested_owners_and_stable_paths() {
    use shadows_core::{DesignOp, PartContent, PartId, PartLayer, PlanOp};
    let (_tmp, storage) = fixture().await;
    let project = ProjectId::from_literal("p");
    let plans = AppCore::plans_for_tests(storage.clone());
    let design = AppCore::design_for_tests(storage.clone());
    let backend = PartId::generate();
    let db = PartId::generate();
    let layer = |name: &str, place: &str| PartLayer {
        layer: name.into(),
        place: place.into(),
        not_used: None,
    };
    let backend_layers = [
        "domain",
        "application",
        "interface",
        "infrastructure",
        "composition",
    ]
    .iter()
    .map(|id| layer(id, &format!("backend/{id}")))
    .collect();
    plans
        .edit_design(
            "layout".into(),
            &project,
            design.vision(&project).await.unwrap().revision,
            vec![
                DesignOp::PartCreate {
                    id: backend.clone(),
                    parent: None,
                    before: None,
                    content: PartContent {
                        title: "Backend".into(),
                        kind: Some("backend".into()),
                        folder: "backend".into(),
                        layers: backend_layers,
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
                        layers: vec![
                            layer("schema", "backend/database/schema"),
                            layer("migrations", "backend/database/migrations"),
                            layer("connection", "backend/config/database.php"),
                        ],
                        ..Default::default()
                    },
                },
            ],
        )
        .await
        .unwrap();
    let workflow = WorkflowId::from_literal(SOURCE);
    let PlanOp::TaskAdd { mut task } = task(1) else {
        unreachable!()
    };
    task.reads = vec!["backend/config/database.php".into(), "docs/notes.md".into()];
    task.writes = vec![
        "backend/database/migrations/x.php".into(),
        "docs/notes.md".into(),
    ];
    plans
        .edit_plan_for_test(
            &command("task"),
            &Writer::Person,
            None,
            &workflow,
            0,
            &[PlanOp::TaskAdd { task }],
        )
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("approve"), &workflow, 1)
        .await
        .unwrap();
    let context = plans
        .task_context(&grant(&storage).await, Some(&workflow), 1)
        .await
        .unwrap();
    assert_eq!(context.unowned_paths, vec!["docs/notes.md"]);
    let backend = context.parts.iter().find(|p| p.part.id == backend).unwrap();
    let db = context.parts.iter().find(|p| p.part.id == db).unwrap();
    assert!(backend.part.rules.iter().any(|r| r.id == "S10"));
    assert!(db.part.rules.iter().any(|r| r.id == "S4"));
    assert!(backend.paths.iter().all(|p| !p.path.contains("migrations")));
    assert!(
        db.paths
            .iter()
            .any(|p| p.via == OwnedVia::Layer("connection".into()))
    );
    assert!(db.paths.iter().any(|p| p.via == OwnedVia::Folder));
    assert_eq!(backend.part.layers.len(), 5);
    storage.close().await.unwrap();
}

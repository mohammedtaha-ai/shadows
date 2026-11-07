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
            .task_context(&grant, Some(&workflow), 1, None)
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
        .task_context(&grant, Some(&workflow), 1, None)
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
            .task_context(&grant, Some(&workflow), 1, None)
            .await
            .unwrap()
            .task
            .title,
        "Task 1"
    );

    let err = plans
        .task_context(&grant, Some(&workflow), 99, None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("T99"));
    assert!(plans.task_context(&grant, None, 1, None).await.is_err());
    let mut foreign = grant.clone();
    foreign.project_id = ProjectId::from_literal("foreign");
    assert!(
        plans
            .task_context(&foreign, Some(&workflow), 1, None)
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
        .task_context(&grant(&storage).await, Some(&workflow), 1, None)
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

#[tokio::test]
async fn selected_phase_context_keeps_all_acceptance_and_exact_dependency_content() {
    use cross_plan::{TARGET, TARGET_PLAN};
    use shadows_core::{AcceptanceItem, PhaseInput, PhaseRef, PlanId, PlanOp};
    let (_tmp, storage) = fixture().await;
    cross_plan::approve_roadmap(&storage, "p", &[TARGET_PLAN]).await;
    let plans = AppCore::plans_for_tests(storage.clone());
    let source = WorkflowId::from_literal(SOURCE);
    let target = WorkflowId::from_literal(TARGET);
    let PlanOp::TaskAdd { mut task } = task(1) else {
        unreachable!()
    };
    task.acceptance.push(AcceptanceItem {
        number: 2,
        text: "The whole task works".into(),
    });
    plans
        .edit_plan_for_test(
            &command("source"),
            &Writer::Person,
            None,
            &source,
            0,
            &[PlanOp::TaskAdd { task }],
        )
        .await
        .unwrap();
    plans
        .edit_plan_for_test(
            &command("target"),
            &Writer::Person,
            None,
            &target,
            0,
            &[cross_plan::task(3)],
        )
        .await
        .unwrap();
    let input = |title: &str| PhaseInput {
        id: None,
        title: title.into(),
        goal: title.into(),
        acceptance: vec![1],
        waits_for: vec![],
    };
    plans
        .edit_plan_for_test(
            &command("target-phase"),
            &Writer::Person,
            None,
            &target,
            1,
            &[PlanOp::PhasesSet {
                task: 3,
                phases: vec![input("Prepare database")],
            }],
        )
        .await
        .unwrap();
    let target_phase = plans.get(&target).await.unwrap().tasks[0].content.phases[0]
        .id
        .clone();
    plans
        .approve_plan_for_test(&command("approve-target"), &target, 2)
        .await
        .unwrap();
    let mut phase = input("Build");
    phase.waits_for.push(PhaseRef::Plan {
        plan_id: PlanId::from_literal(TARGET_PLAN),
        version: 1,
        task: 3,
        phase: target_phase.clone(),
    });
    plans
        .edit_plan_for_test(
            &command("source-phase"),
            &Writer::Person,
            None,
            &source,
            1,
            &[PlanOp::PhasesSet {
                task: 1,
                phases: vec![phase],
            }],
        )
        .await
        .unwrap();
    plans
        .approve_plan_for_test(&command("approve-source"), &source, 2)
        .await
        .unwrap();
    let source_phase = plans.get(&source).await.unwrap().tasks[0].content.phases[0]
        .id
        .clone();
    let grant = grant(&storage).await;
    let context = plans
        .task_context(&grant, Some(&source), 1, Some(&source_phase))
        .await
        .unwrap();
    assert_eq!(context.task.acceptance.len(), 2);
    assert_eq!(context.phase.as_ref().unwrap().acceptance, vec![1]);
    let dependency = &context.phase_dependencies[0];
    assert_eq!(dependency.target.as_ref().unwrap().workflow_id, target);
    assert_eq!(dependency.target.as_ref().unwrap().version, 1);
    assert_eq!(dependency.phase.as_ref().unwrap().goal, "Prepare database");
    assert!(dependency.broken.is_none());
    assert!(
        plans
            .task_context(&grant, Some(&source), 1, Some(&target_phase))
            .await
            .is_err()
    );
    storage.close().await.unwrap();
}

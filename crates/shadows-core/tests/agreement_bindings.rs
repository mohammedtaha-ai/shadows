use serde_json::json;
use shadows_core::testing::apply;
use shadows_core::{AgreementBinding, PlanOp};
#[path = "fixtures/cross_plan.rs"]
#[allow(dead_code)]
mod cross_plan;

#[tokio::test]
async fn binding_storage_checks_agreed_party_and_exact_operations_atomically() {
    use shadows_core::testing::{LiveHandles, Runtime, Writer, acp};
    use shadows_core::{
        AgreementContent, AgreementParty, AgreementRole, AppCore, CoreParts, DesignOp, PartContent,
        PartId, WorkflowId,
    };
    use std::sync::Arc;
    let (tmp, storage) = cross_plan::fixture().await;
    let storage = Arc::new(storage);
    let core = AppCore::assemble(CoreParts {
        storage: storage.clone(),
        runtime: Arc::new(Runtime::start(storage.clone()).await.unwrap().0),
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(32).0,
        ui: tokio::sync::broadcast::channel(32).0,
        mcp_url: acp::MCP_URL.into(),
    });
    let workflow = WorkflowId::from_literal(cross_plan::SOURCE);
    let project = core.plans().get(&workflow).await.unwrap().project_id;
    let part = PartId::generate();
    core.design()
        .edit(
            "part".into(),
            &project,
            0,
            vec![DesignOp::PartCreate {
                id: part.clone(),
                parent: None,
                before: None,
                content: PartContent {
                    title: "Web".into(),
                    responsibility: "Login".into(),
                    design: "".into(),
                    kind: None,
                },
            }],
        )
        .await
        .unwrap();
    let operation = "c0000000-0000-4000-8000-000000000001";
    let agreement = core
        .design()
        .start_agreement(
            "agreement".into(),
            &project,
            None,
            Some(AgreementContent {
                capability: "Login".into(),
                purpose: "Login".into(),
                behavior: "".into(),
                acceptance: vec!["Login works".into()],
                parties: vec![AgreementParty {
                    part_id: part.clone(),
                    role: AgreementRole::Provides,
                }],
                openapi: json!({"openapi":"3.1.0","info":{"title":"Login","version":"1"},
                "paths":{"/login":{"post":{"x-shadows-operation-id":operation,
                    "responses":{"200":{"description":"OK"}}}}}}),
            }),
            None,
        )
        .await
        .unwrap();
    let binding = AgreementBinding {
        task: 4,
        agreement_id: agreement.agreement_id.clone(),
        version: 1,
        part_id: part,
        role: AgreementRole::Provides,
        operations: vec![operation.into()],
    };
    let ctx = cross_plan::command("bind");
    let ops = [
        cross_plan::task(4),
        PlanOp::BindingPut {
            binding: binding.clone(),
        },
    ];
    assert!(
        storage
            .edit_plan(&ctx, &Writer::Person, None, &workflow, 0, &ops)
            .await
            .is_err()
    );
    assert!(core.plans().get(&workflow).await.unwrap().tasks.is_empty());
    let db = sqlx::SqlitePool::connect(&format!(
        "sqlite:{}",
        tmp.path().join("links.sqlite3").display()
    ))
    .await
    .unwrap();
    sqlx::query(
        "UPDATE agreement_version SET state='Agreed',agreed_at='test' WHERE agreement_id=?",
    )
    .bind(agreement.agreement_id.as_str())
    .execute(&db)
    .await
    .unwrap();
    storage
        .edit_plan(&ctx, &Writer::Person, None, &workflow, 0, &ops)
        .await
        .unwrap();
    assert_eq!(
        core.plans().get(&workflow).await.unwrap().bindings,
        vec![binding.clone()]
    );
    let mut invalid = binding.clone();
    invalid.operations = vec!["absent".into()];
    assert!(
        storage
            .edit_plan(
                &cross_plan::command("invalid"),
                &Writer::Person,
                None,
                &workflow,
                1,
                &[PlanOp::BindingPut { binding: invalid }]
            )
            .await
            .is_err()
    );
    assert_eq!(core.plans().get(&workflow).await.unwrap().revision, 1);
    core.plans()
        .approve("freeze".into(), &workflow, 1)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE task_agreement_binding SET operations_json='[]' WHERE workflow_id=?")
            .bind(workflow.as_str())
            .execute(&db)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM task_agreement_binding WHERE workflow_id=?")
            .bind(workflow.as_str())
            .execute(&db)
            .await
            .is_err()
    );
    assert!(
        sqlx::query(
            "INSERT INTO task_agreement_binding SELECT * FROM task_agreement_binding \
        WHERE workflow_id=?"
        )
        .bind(workflow.as_str())
        .execute(&db)
        .await
        .is_err()
    );
    db.close().await;
}

#[test]
fn bindings_are_exact_version_plan_content_and_task_removal_cleans_them() {
    let binding: AgreementBinding = serde_json::from_value(json!({
        "task":1,"agreement_id":"a0000000-0000-4000-8000-000000000001","version":1,
        "part_id":"b0000000-0000-4000-8000-000000000001","role":"uses",
        "operations":["c0000000-0000-4000-8000-000000000001"]
    }))
    .unwrap();
    let content: shadows_core::PlanContent = serde_json::from_value(json!({
        "title":"Login","goal":"Login","tasks":{"1":{"number":1,"title":"UI",
        "goal":"Login","reads":[],"writes":[],"acceptance":[]}},"links":[]
    }))
    .unwrap();
    assert!(
        serde_json::to_value(&content)
            .unwrap()
            .get("bindings")
            .is_none()
    );
    let result = apply(
        &content,
        &[PlanOp::BindingPut {
            binding: binding.clone(),
        }],
    )
    .unwrap();
    assert_eq!(result.content.bindings, vec![binding]);
    let removed = apply(&result.content, &[PlanOp::TaskRemove { number: 1 }]).unwrap();
    assert!(removed.content.bindings.is_empty());
    let foreign = json!({"op":"binding_put","binding": {
        "task":9,"agreement_id":"a0000000-0000-4000-8000-000000000001","version":1,
        "part_id":"b0000000-0000-4000-8000-000000000001","role":"uses","operations":[]
    }});
    assert!(apply(&content, &[serde_json::from_value(foreign).unwrap()]).is_err());
}

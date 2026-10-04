//! Two independently pinned plans through proposal, review, agreement and adoption.
use serde_json::json;
use shadows_core::testing::acp;
use shadows_core::{
    AgreementBinding, AgreementContent, AgreementParty, AgreementRole, CoreError, DesignOp,
    PartContent, PartId, PlanOp, StorageError,
};
#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

#[tokio::test]
async fn two_plans_adopt_independently_and_retain_frozen_pins_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let a = app::test_app_at(temp.path()).await;
    let provider = PartId::generate();
    let consumer = PartId::generate();
    let parts = vec![
        (provider.clone(), "Backend"),
        (consumer.clone(), "Frontend"),
    ];
    a.core
        .design()
        .edit(
            "parts".into(),
            &a.project,
            0,
            parts
                .into_iter()
                .map(|(id, title)| DesignOp::PartCreate {
                    id,
                    parent: None,
                    before: None,
                    content: PartContent {
                        title: title.into(),
                        responsibility: "Login".into(),
                        design: "".into(),
                        kind: None,
                    },
                })
                .collect(),
        )
        .await
        .unwrap();
    let op = "c0000000-0000-4000-8000-000000000001";
    let content = AgreementContent {
        capability: "Login".into(),
        purpose: "Login".into(),
        behavior: "Invalid credentials do not create sessions".into(),
        acceptance: vec!["Valid credentials return a session".into()],
        parties: vec![
            AgreementParty {
                part_id: provider.clone(),
                role: AgreementRole::Provides,
            },
            AgreementParty {
                part_id: consumer.clone(),
                role: AgreementRole::Uses,
            },
        ],
        openapi: json!({"openapi":"3.1.0","info":{"title":"Login","version":"1"},
            "paths":{"/login":{"post":{"x-shadows-operation-id":op,
                "responses":{"200":{"description":"Session"}}}}}}),
    };
    let first = a
        .core
        .design()
        .start_agreement("contract".into(), &a.project, None, Some(content), None)
        .await
        .unwrap();
    let review = a
        .core
        .design()
        .review_agreement(&a.project, &first.agreement_id)
        .await
        .unwrap();
    a.core
        .design()
        .agree_agreement(
            "agree-v1".into(),
            &a.project,
            &first.agreement_id,
            0,
            review.review_id,
        )
        .await
        .unwrap();
    let front_thread = a
        .storage
        .create_planning_thread(
            &app::ctx("front", "thread.create"),
            &a.project,
            "Frontend",
            "claude-code",
        )
        .await
        .unwrap()
        .id;
    let backend = plan::draft(&a).await;
    let frontend = plan::draft_on(&a, &front_thread, "frontend").await;
    let binding = |part, role, version| AgreementBinding {
        task: 1,
        agreement_id: first.agreement_id.clone(),
        version,
        part_id: part,
        role,
        operations: vec![op.into()],
    };
    for (workflow, part, role) in [
        (&backend.workflow_id, provider, AgreementRole::Provides),
        (&frontend.workflow_id, consumer.clone(), AgreementRole::Uses),
    ] {
        plan::edit(
            &a,
            workflow,
            0,
            &[
                plan::add(1),
                PlanOp::BindingPut {
                    binding: binding(part, role, 1),
                },
            ],
        )
        .await;
        a.core
            .plans()
            .approve(format!("approve-{workflow}"), workflow, 1)
            .await
            .unwrap();
    }
    let old_front = a
        .core
        .plans()
        .get(&frontend.workflow_id)
        .await
        .unwrap()
        .content();
    let old_back = a
        .core
        .plans()
        .get(&backend.workflow_id)
        .await
        .unwrap()
        .content();
    let second = a
        .core
        .design()
        .start_agreement(
            "v2".into(),
            &a.project,
            Some(&first.agreement_id),
            None,
            Some("Add retry behavior".into()),
        )
        .await
        .unwrap();
    let mut content = second.content;
    content.behavior = "Rate limit repeated login failures".into();
    a.core
        .design()
        .edit_agreement(
            "edit-v2".into(),
            &a.project,
            &first.agreement_id,
            2,
            0,
            content,
        )
        .await
        .unwrap();
    let review = a
        .core
        .design()
        .review_agreement(&a.project, &first.agreement_id)
        .await
        .unwrap();
    assert_eq!(
        review
            .participants
            .iter()
            .filter(|p| p.affected && p.participant.current)
            .count(),
        2
    );
    assert!(
        review
            .participants
            .iter()
            .all(|p| p.execution == "Not recorded")
    );
    let continued = plan::draft_on(&a, &front_thread, "continue-front").await;
    assert_eq!(
        a.core
            .plans()
            .get(&continued.workflow_id)
            .await
            .unwrap()
            .bindings[0]
            .version,
        1
    );
    assert!(matches!(
        a.core
            .design()
            .agree_agreement(
                "stale".into(),
                &a.project,
                &first.agreement_id,
                1,
                review.review_id
            )
            .await,
        Err(CoreError::Storage(StorageError::RevisionConflict { .. }))
    ));
    let review = a
        .core
        .design()
        .review_agreement(&a.project, &first.agreement_id)
        .await
        .unwrap();
    assert_eq!(
        review
            .participants
            .iter()
            .filter(|p| !p.participant.current)
            .count(),
        1
    );
    a.core
        .design()
        .agree_agreement(
            "agree-v2".into(),
            &a.project,
            &first.agreement_id,
            1,
            review.review_id,
        )
        .await
        .unwrap();
    assert_eq!(
        a.core
            .plans()
            .get(&continued.workflow_id)
            .await
            .unwrap()
            .bindings[0]
            .version,
        1
    );
    a.core
        .plans()
        .edit_bindings(
            "adopt-v2".into(),
            &continued.workflow_id,
            0,
            vec![PlanOp::BindingPut {
                binding: binding(consumer, AgreementRole::Uses, 2),
            }],
        )
        .await
        .unwrap();
    assert_eq!(
        a.core
            .plans()
            .get(&continued.workflow_id)
            .await
            .unwrap()
            .bindings[0]
            .version,
        2
    );
    assert_eq!(
        a.core
            .plans()
            .get(&frontend.workflow_id)
            .await
            .unwrap()
            .content(),
        old_front
    );
    assert_eq!(
        a.core
            .plans()
            .get(&backend.workflow_id)
            .await
            .unwrap()
            .content(),
        old_back
    );
    let third = a
        .core
        .design()
        .start_agreement(
            "v3".into(),
            &a.project,
            Some(&first.agreement_id),
            None,
            Some("Rename capability".into()),
        )
        .await
        .unwrap();
    let mut renamed = third.content;
    renamed.capability = "Authentication".into();
    a.core
        .design()
        .edit_agreement(
            "rename-v3".into(),
            &a.project,
            &first.agreement_id,
            3,
            0,
            renamed,
        )
        .await
        .unwrap();
    let third_review = a
        .core
        .design()
        .review_agreement(&a.project, &first.agreement_id)
        .await
        .unwrap();
    assert!(
        third_review
            .participants
            .iter()
            .any(|p| p.participant.current && p.participant.binding.version == 1 && p.affected)
    );
    assert!(
        third_review
            .participants
            .iter()
            .any(|p| p.participant.current && p.participant.binding.version == 2 && !p.affected)
    );
    let project = a.project.clone();
    drop(a);
    let reopened = app::test_app_at(temp.path()).await;
    assert_eq!(
        reopened
            .core
            .design()
            .agreement(&project, &first.agreement_id, Some(1))
            .await
            .unwrap()
            .version,
        1
    );
    assert_eq!(
        reopened
            .core
            .plans()
            .get(&continued.workflow_id)
            .await
            .unwrap()
            .bindings[0]
            .version,
        2
    );
    assert_eq!(
        reopened
            .core
            .plans()
            .get(&backend.workflow_id)
            .await
            .unwrap()
            .bindings[0]
            .version,
        1
    );
}

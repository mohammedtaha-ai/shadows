use shadows_core::testing::{LiveHandles, Runtime, Storage, acp};
use shadows_core::{AgreementContent, AgreementState, AppCore, CoreError, CoreParts, StorageError};
use std::sync::Arc;

#[tokio::test]
async fn agreement_draft_replays_exact_result_and_refuses_stale_edit() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let core = AppCore::assemble(CoreParts {
        storage: storage.clone(),
        runtime: Arc::new(Runtime::start(storage.clone()).await.unwrap().0),
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(32).0,
        ui: tokio::sync::broadcast::channel(32).0,
        mcp_url: acp::MCP_URL.into(),
    });
    let project = core
        .projects()
        .create(
            "project".into(),
            "agreements",
            "Agreements",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    let content = AgreementContent {
        capability: "Login".into(),
        purpose: "Sign in".into(),
        behavior: "".into(),
        acceptance: vec![],
        parties: vec![],
        openapi: serde_json::json!({}),
    };
    let first = core
        .design()
        .start_agreement("start".into(), &project, None, Some(content.clone()), None)
        .await
        .unwrap();
    assert_eq!(first.version, 1);
    assert_eq!(first.revision, 0);
    assert_eq!(first.state, AgreementState::Draft);
    assert!(!first.issues.is_empty());
    let mut changed = content.clone();
    changed.purpose = "Changed".into();
    let edited = core
        .design()
        .edit_agreement(
            "edit".into(),
            &project,
            &first.agreement_id,
            0,
            changed.clone(),
        )
        .await
        .unwrap();
    assert_eq!(edited.revision, 1);
    let replay = core
        .design()
        .start_agreement("start".into(), &project, None, Some(content), None)
        .await
        .unwrap();
    assert_eq!(replay.agreement_id, first.agreement_id);
    assert_eq!(replay.revision, 0);
    assert_eq!(replay.content.purpose, "Sign in");
    let existing = core
        .design()
        .start_agreement(
            "same".into(),
            &project,
            Some(&first.agreement_id),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(existing.revision, 1);
    assert!(matches!(
        core.design()
            .edit_agreement(
                "stale".into(),
                &project,
                &first.agreement_id,
                0,
                changed.clone()
            )
            .await,
        Err(CoreError::Storage(StorageError::RevisionConflict { .. }))
    ));
    assert!(matches!(
        core.design()
            .edit_agreement(
                "edit".into(),
                &project,
                &first.agreement_id,
                0,
                first.content
            )
            .await,
        Err(CoreError::Storage(StorageError::CommandConflict))
    ));
    assert_eq!(core.design().agreements(&project).await.unwrap().len(), 1);
    let exact = core
        .design()
        .agreement(&project, &first.agreement_id, Some(1))
        .await
        .unwrap();
    assert_eq!(exact.content.purpose, "Changed");
    let reader =
        sqlx::SqlitePool::connect(&format!("sqlite:{}", tmp.path().join("test.db").display()))
            .await
            .unwrap();
    sqlx::query(
        "UPDATE agreement_version SET state='Agreed',agreed_at='test' WHERE agreement_id=?",
    )
    .bind(first.agreement_id.as_str())
    .execute(&reader)
    .await
    .unwrap();
    assert!(
        sqlx::query("UPDATE agreement_version SET content_json='{}' WHERE agreement_id=?")
            .bind(first.agreement_id.as_str())
            .execute(&reader)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM agreement_version WHERE agreement_id=?")
            .bind(first.agreement_id.as_str())
            .execute(&reader)
            .await
            .is_err()
    );
    assert!(
        core.design()
            .start_agreement(
                "v2-no-reason".into(),
                &project,
                Some(&first.agreement_id),
                None,
                None
            )
            .await
            .is_err()
    );
    let start_a = core.design().start_agreement(
        "v2".into(),
        &project,
        Some(&first.agreement_id),
        None,
        Some("Improve login".into()),
    );
    let start_b = core.design().start_agreement(
        "competing-v2".into(),
        &project,
        Some(&first.agreement_id),
        None,
        Some("Concurrent continuation".into()),
    );
    let (v2, competing) = tokio::join!(start_a, start_b);
    let v2 = v2.unwrap();
    assert_eq!(competing.unwrap().version, v2.version);
    assert_eq!(v2.version, 2);
    assert_eq!(v2.content.purpose, "Changed");
    let mut with_operation = v2.content.clone();
    with_operation.openapi = serde_json::json!({"paths":{"/login":{"post":{
        "x-shadows-operation-id":"c0000000-0000-4000-8000-000000000001"
    }}}});
    core.design()
        .edit_agreement(
            "operation".into(),
            &project,
            &first.agreement_id,
            0,
            with_operation.clone(),
        )
        .await
        .unwrap();
    core.design()
        .edit_agreement(
            "remove-operation".into(),
            &project,
            &first.agreement_id,
            1,
            v2.content,
        )
        .await
        .unwrap();
    assert!(
        core.design()
            .edit_agreement(
                "reuse-operation".into(),
                &project,
                &first.agreement_id,
                2,
                with_operation
            )
            .await
            .is_err()
    );
    reader.close().await;
    let grant = core
        .grants()
        .issue("grant".into(), &project)
        .await
        .unwrap()
        .grant;
    let agent_content = core
        .design()
        .agreement(&project, &first.agreement_id, Some(1))
        .await
        .unwrap()
        .content;
    let proposed = core
        .design()
        .start_agreement_for(
            &grant,
            "agent-start".into(),
            None,
            Some(agent_content.clone()),
            None,
        )
        .await
        .unwrap();
    core.grants()
        .revoke("revoke-grant".into(), &grant.id)
        .await
        .unwrap();
    assert!(
        core.design()
            .start_agreement_for(
                &grant,
                "agent-start".into(),
                None,
                Some(agent_content.clone()),
                None
            )
            .await
            .is_err()
    );
    assert!(
        core.design()
            .edit_agreement_for(
                &grant,
                "revoked-edit".into(),
                &proposed.agreement_id,
                0,
                agent_content
            )
            .await
            .is_err()
    );
    assert_eq!(
        core.design()
            .agreement(&project, &proposed.agreement_id, None)
            .await
            .unwrap()
            .revision,
        0
    );
}

use serde_json::json;
use shadows_core::testing::{LiveHandles, Storage, acp};
use shadows_core::{
    AgreementContent, AgreementParty, AgreementRole, AgreementState, AppCore, CoreError, CoreParts,
    DesignOp, PartContent, PartId, StorageError,
};
use std::sync::Arc;

#[tokio::test]
async fn agreement_review_detects_stale_part_and_agrees_without_implicit_adoption() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("review.db")).await.unwrap());
    let core = AppCore::assemble(CoreParts {
        operations: shadows_core::AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime: Arc::new(
            shadows_core::AppCore::runtime_for_tests(storage.clone())
                .await
                .unwrap()
                .0,
        ),
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(32).0,
        ui: tokio::sync::broadcast::channel(32).0,
        mcp_url: acp::MCP_URL.into(),
    });
    let project = core
        .project_lifecycle()
        .create(
            "project".into(),
            "review",
            "Review",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    let part = PartId::generate();
    shadows_core::AppCore::design_for_tests(storage.clone())
        .seed_map_for_tests(&project)
        .await
        .unwrap();
    let part_content = PartContent {
        serves: vec![],
        folders: vec![],
        files: vec![],
        contract: Default::default(),
        title: "API".into(),
        responsibility: "Login".into(),
        design: "".into(),
        kind: None,
    };
    core.plans()
        .edit_design(
            "part".into(),
            &project,
            0,
            vec![DesignOp::PartCreate {
                id: part.clone(),
                parent: None,
                before: None,
                content: part_content.clone(),
            }],
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let first=core.design().start_agreement("start".into(),&project,None,Some(AgreementContent {
        capability:"Login".into(),purpose:"Login".into(),behavior:"Credentials".into(),
        acceptance:vec!["Returns session".into()],parties:vec![AgreementParty {
            part_id:part.clone(),role:AgreementRole::Provides,
        }],openapi:json!({"openapi":"3.1.0","info":{"title":"API","version":"1"},
            "paths":{"/login":{"post":{"x-shadows-operation-id":
                "c0000000-0000-4000-8000-000000000001","responses":{"200":{"description":"OK"}}}}}})
    }),None).await.map_err(shadows_core::CoreError::from).unwrap();
    let review = core
        .plans()
        .review_agreement(&project, &first.agreement_id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(review.compatibility, "Needs review");
    assert_eq!(review.parties.len(), 1);
    assert!(review.participants.is_empty());
    core.plans()
        .edit_design(
            "rename".into(),
            &project,
            1,
            vec![DesignOp::PartPut {
                id: part,
                content: PartContent {
                    title: "Renamed".into(),
                    ..part_content
                },
            }],
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(matches!(
        core.plans()
            .agree_agreement(
                "stale".into(),
                &project,
                &first.agreement_id,
                0,
                review.review_id
            )
            .await
            .map_err(shadows_core::CoreError::from),
        Err(CoreError::Storage(StorageError::RevisionConflict { .. }))
    ));
    let fresh = core
        .plans()
        .review_agreement(&project, &first.agreement_id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let agreed = core
        .plans()
        .agree_agreement(
            "agree".into(),
            &project,
            &first.agreement_id,
            0,
            fresh.review_id.clone(),
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(agreed.state, AgreementState::Agreed);
    let v2 = core
        .design()
        .start_agreement(
            "v2".into(),
            &project,
            Some(&first.agreement_id),
            None,
            Some("New behavior".into()),
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let mut content = v2.content;
    content.behavior = "Changed behavior".into();
    // A request built on v1 at revision 0 must not land on v2, which is
    // also at revision 0: it names its version, and v1 is Frozen.
    let stale = core
        .design()
        .edit_agreement(
            "built-on-v1".into(),
            &project,
            &first.agreement_id,
            1,
            0,
            content.clone(),
        )
        .await
        .map_err(shadows_core::CoreError::from);
    assert!(
        matches!(&stale, Err(CoreError::Refused { message, .. }) if message.contains("read v2")),
        "{stale:?}"
    );
    core.design()
        .edit_agreement(
            "change".into(),
            &project,
            &first.agreement_id,
            2,
            0,
            content,
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let replay = core
        .plans()
        .agree_agreement(
            "agree".into(),
            &project,
            &first.agreement_id,
            0,
            fresh.review_id,
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(replay.version, 1);
    assert_eq!(
        core.design()
            .agreement(&project, &first.agreement_id, Some(1))
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .content
            .behavior,
        "Credentials"
    );
}

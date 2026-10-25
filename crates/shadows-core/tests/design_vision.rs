use std::sync::Arc;

use shadows_core::testing::{LiveHandles, Storage, acp};
use shadows_core::{
    AdditionalPart, AppCore, CoreError, CoreParts, DesignOp, ProjectId, StandardRule,
    StandardsAdditions, StorageError, VisionContent,
};

#[path = "design_vision/standards.rs"]
mod standards;

#[path = "design_vision/items.rs"]
mod items;

#[path = "design_vision/proposals.rs"]
mod proposals;

fn content(text: &str) -> VisionContent {
    VisionContent {
        purpose: vec![shadows_core::VisionItem {
            id: "PU1".into(),
            text: text.into(),
            approved_from: None,
            revision: 1,
        }],
        ..VisionContent::default()
    }
}

#[tokio::test]
async fn vision_edit_is_atomic_replayable_and_revision_checked() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&tmp.path().join("test.sqlite3"))
            .await
            .unwrap(),
    );
    let runtime = Arc::new(
        shadows_core::AppCore::runtime_for_tests(storage.clone())
            .await
            .unwrap()
            .0,
    );
    let core = AppCore::assemble(CoreParts {
        operations: shadows_core::AppCore::operations_for_tests(storage.clone()),
        secrets: shadows_core::testing::secrets(storage.clone(), None),
        storage: storage.clone(),
        runtime,
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
            "vision",
            "Vision",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    assert_eq!(core.design().vision(&project).await.unwrap().revision, 0);
    let first = vec![DesignOp::VisionItemAdd {
        field: shadows_core::VisionField::Purpose,
        text: " رؤية ".into(),
    }];
    assert_eq!(
        core.plans()
            .edit_design("first".into(), &project, 0, first.clone())
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .revision,
        1
    );
    assert_eq!(
        core.design().vision(&project).await.unwrap().content,
        content(" رؤية ")
    );
    core.plans()
        .edit_design(
            "second".into(),
            &project,
            1,
            vec![DesignOp::VisionItemPut {
                id: "PU1".into(),
                text: "ثانية".into(),
            }],
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(
        core.plans()
            .edit_design("first".into(), &project, 0, first)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .revision,
        1
    );
    assert!(matches!(
        core.plans()
            .edit_design(
                "first".into(),
                &project,
                0,
                vec![DesignOp::VisionItemPut {
                    id: "PU1".into(),
                    text: "changed".into()
                }]
            )
            .await
            .map_err(shadows_core::CoreError::from),
        Err(CoreError::Storage(StorageError::CommandConflict))
    ));
    let other = core
        .project_lifecycle()
        .create(
            "other".into(),
            "other",
            "Other",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    let ops = vec![DesignOp::VisionItemAdd {
        field: shadows_core::VisionField::Purpose,
        text: "competing".into(),
    }];
    let (a, b) = tokio::join!(
        core.plans().edit_design("a".into(), &other, 0, ops.clone()),
        core.plans().edit_design("b".into(), &other, 0, ops)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let loser = if a.is_err() { a } else { b };
    assert!(matches!(
        loser.map_err(CoreError::from),
        Err(CoreError::Storage(StorageError::RevisionConflict {
            current: 1,
            ..
        }))
    ));
    core.project_lifecycle()
        .remove("remove".into(), &project)
        .await
        .unwrap();
    let before = shadows_core::Events::current_cursor_for_test(&storage)
        .await
        .unwrap();
    assert!(core.design().vision(&project).await.is_err());
    assert!(
        core.plans()
            .edit_design(
                "removed".into(),
                &project,
                2,
                vec![DesignOp::VisionItemPut {
                    id: "PU1".into(),
                    text: "bad".into()
                }]
            )
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    assert_eq!(
        shadows_core::Events::current_cursor_for_test(&storage)
            .await
            .unwrap()
            .0,
        before.0
    );
    let events: Vec<String> = sqlx::query_scalar(
        r#"SELECT payload_json FROM durable_event
               WHERE kind = 'ProjectDesignChanged' AND project_id = ? ORDER BY seq"#,
    )
    .bind(project.as_str())
    .fetch_all(storage.reader())
    .await
    .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&events[0]).unwrap(),
        serde_json::json!({
            "project_id": project,
            "revision": 1,
            "changed_parts": [],
            "changed_outcomes": [],
            "vision_changed": true
        })
    );
    // A late journal failure must roll back content, result and command together.
    sqlx::query(
        r#"CREATE TRIGGER refuse_design_event BEFORE INSERT ON durable_event
           WHEN NEW.kind = 'ProjectDesignChanged'
           BEGIN SELECT RAISE(ABORT, 'test refusal'); END"#,
    )
    .execute(storage.reader())
    .await
    .unwrap();
    let before_view = core.design().vision(&other).await.unwrap();
    let before_cursor = shadows_core::Events::current_cursor_for_test(&storage)
        .await
        .unwrap()
        .0;
    assert!(
        core.plans()
            .edit_design(
                "rollback".into(),
                &other,
                1,
                vec![DesignOp::VisionItemPut {
                    id: "PU1".into(),
                    text: "must roll back".into()
                }]
            )
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    assert_eq!(core.design().vision(&other).await.unwrap(), before_view);
    assert_eq!(
        shadows_core::Events::current_cursor_for_test(&storage)
            .await
            .unwrap()
            .0,
        before_cursor
    );
    let results: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM design_command_result")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(results, 3);
    sqlx::query("DROP TRIGGER refuse_design_event")
        .execute(storage.reader())
        .await
        .unwrap();
    let batch = vec![
        DesignOp::VisionItemPut {
            id: "PU1".into(),
            text: "first put".into(),
        },
        DesignOp::VisionItemPut {
            id: "PU1".into(),
            text: "last put".into(),
        },
    ];
    assert_eq!(
        core.plans()
            .edit_design("batch".into(), &other, 1, batch)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .revision,
        2
    );
    assert_eq!(
        core.design().vision(&other).await.unwrap().content,
        VisionContent {
            purpose: vec![shadows_core::VisionItem {
                revision: 3,
                ..content("last put").purpose.remove(0)
            }],
            ..VisionContent::default()
        }
    );
    let revisions: (i64, i64) = sqlx::query_as(
        "SELECT revision, vision_revision FROM design_workspace WHERE project_id = ?",
    )
    .bind(other.as_str())
    .fetch_one(storage.reader())
    .await
    .unwrap();
    assert_eq!(revisions, (2, 2));
    assert!(
        sqlx::query("UPDATE design_command_result SET result_json = '{}' ")
            .execute(storage.reader())
            .await
            .is_err()
    );
    core.shut_down(std::time::Duration::from_secs(2), std::future::pending())
        .await
        .unwrap();
}

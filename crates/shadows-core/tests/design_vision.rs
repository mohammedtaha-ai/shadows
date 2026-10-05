use std::sync::Arc;

use shadows_core::testing::{LiveHandles, Runtime, Storage, acp};
use shadows_core::{AppCore, CoreError, CoreParts, DesignOp, StorageError, VisionContent};

fn content(text: &str) -> VisionContent {
    VisionContent {
        purpose: text.into(),
        users: " مطورون ".into(),
        goals: "أهداف\n".into(),
        boundaries: "حدود".into(),
        technical_direction: "Rust + React".into(),
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
    let runtime = Arc::new(Runtime::start(storage.clone()).await.unwrap().0);
    let core = AppCore::assemble(CoreParts {
        storage: storage.clone(),
        runtime,
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
            "vision",
            "Vision",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    assert_eq!(core.design().vision(&project).await.unwrap().revision, 0);
    let first = vec![DesignOp::VisionPut {
        content: content(" رؤية "),
    }];
    assert_eq!(
        core.design()
            .edit("first".into(), &project, 0, first.clone())
            .await
            .unwrap()
            .revision,
        1
    );
    assert_eq!(
        core.design().vision(&project).await.unwrap().content,
        content(" رؤية ")
    );
    core.design()
        .edit(
            "second".into(),
            &project,
            1,
            vec![DesignOp::VisionPut {
                content: content("ثانية"),
            }],
        )
        .await
        .unwrap();
    assert_eq!(
        core.design()
            .edit("first".into(), &project, 0, first)
            .await
            .unwrap()
            .revision,
        1
    );
    assert!(matches!(
        core.design()
            .edit(
                "first".into(),
                &project,
                0,
                vec![DesignOp::VisionPut {
                    content: content("changed")
                }]
            )
            .await,
        Err(CoreError::Storage(StorageError::CommandConflict))
    ));
    let other = core
        .projects()
        .create(
            "other".into(),
            "other",
            "Other",
            tmp.path().to_str().unwrap(),
        )
        .await
        .unwrap()
        .id;
    let ops = vec![DesignOp::VisionPut {
        content: content("competing"),
    }];
    let (a, b) = tokio::join!(
        core.design().edit("a".into(), &other, 0, ops.clone()),
        core.design().edit("b".into(), &other, 0, ops)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let loser = if a.is_err() { a } else { b };
    assert!(matches!(
        loser,
        Err(CoreError::Storage(StorageError::RevisionConflict {
            current: 1,
            ..
        }))
    ));
    core.projects()
        .remove("remove".into(), &project)
        .await
        .unwrap();
    let before = storage.current_cursor().await.unwrap();
    assert!(core.design().vision(&project).await.is_err());
    assert!(
        core.design()
            .edit(
                "removed".into(),
                &project,
                2,
                vec![DesignOp::VisionPut {
                    content: content("bad")
                }]
            )
            .await
            .is_err()
    );
    assert_eq!(storage.current_cursor().await.unwrap().0, before.0);
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
    let before_cursor = storage.current_cursor().await.unwrap().0;
    assert!(
        core.design()
            .edit(
                "rollback".into(),
                &other,
                1,
                vec![DesignOp::VisionPut {
                    content: content("must roll back")
                }]
            )
            .await
            .is_err()
    );
    assert_eq!(core.design().vision(&other).await.unwrap(), before_view);
    assert_eq!(storage.current_cursor().await.unwrap().0, before_cursor);
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
        DesignOp::VisionPut {
            content: content("first put"),
        },
        DesignOp::VisionPut {
            content: content("last put"),
        },
    ];
    assert_eq!(
        core.design()
            .edit("batch".into(), &other, 1, batch)
            .await
            .unwrap()
            .revision,
        2
    );
    assert_eq!(
        core.design().vision(&other).await.unwrap().content,
        content("last put")
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

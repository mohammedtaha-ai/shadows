use core_db::{Actor, JournalEntry, Storage};
use core_events::Events;
use core_projects::Projects;
use std::sync::Arc;

#[tokio::test]
async fn project_stream_carries_permissions_changed() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let projects = Projects::new(storage.clone());
    let mut project_ids = Vec::new();
    for slug in ["project", "other"] {
        let directory = tmp.path().join(slug);
        std::fs::create_dir(&directory).unwrap();
        project_ids.push(
            projects
                .create(slug.into(), slug, slug, directory.to_str().unwrap())
                .await
                .unwrap()
                .id,
        );
    }
    let project = project_ids[0].clone();
    let after = Events::current_cursor_for_test(&storage).await.unwrap();
    let ids = [
        project_ids[0].as_str().to_owned(),
        project_ids[1].as_str().to_owned(),
    ];
    storage
        .write_txn(move |conn| {
            Box::pin(async move {
                for (project, state) in [
                    (&ids[0], "waiting"),
                    (&ids[1], "waiting"),
                    (&ids[0], "answered"),
                ] {
                    JournalEntry::append_event(
                        conn,
                        JournalEntry {
                            event_id: format!("{project}-{state}"),
                            kind: "PermissionsChanged".into(),
                            project_id: Some(project.clone()),
                            thread_id: None,
                            operation_id: None,
                            actor: Actor::system(),
                            causation: None,
                            correlation_id: None,
                            payload_json: serde_json::json!({"request_id":"request","state":state})
                                .to_string(),
                        },
                        &Storage::now(),
                    )
                    .await?;
                }
                Ok(())
            })
        })
        .await
        .unwrap();
    let rows = Events::read_project_events_after_for_test(storage.clone(), after, &project, 10)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].seq < rows[1].seq);
    assert_eq!(rows[0].kind, "PermissionsChanged");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&rows[1].payload_json).unwrap()["state"],
        "answered"
    );
    storage.close().await.unwrap();
}

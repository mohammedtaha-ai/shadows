//! Build states from committed pins and the project's folder.
use std::sync::Arc;

use code_record::{BuildState, CodeRecord, Confirmation, DoneRecord, PlannedPin};
use codebase::Codebase;
use core_db::Storage;
use core_design::DesignElement;
use core_plans::{ElementAction, ElementPin, WorkflowId};
use core_projects::{ProjectId, Projects};
use serde_json::json;

fn element(project: &ProjectId, id: &str, revision: i64, deleted: bool) -> DesignElement {
    serde_json::from_value(json!({
        "id": id, "revision": revision, "project_id": project, "part_id": "db",
        "outcomes": ["O1"], "deleted": deleted, "written_by": "planner", "created_at": "now",
        "content": {
            "kind": "table", "name": "users", "purpose": "Users",
            "columns": [{"name": "id", "type_name": "TEXT"}]
        }
    }))
    .unwrap()
}

#[tokio::test]
async fn element_states_read_the_record_and_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("users.sql"), "users").unwrap();
    let db = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&db.path().join("record.sqlite"))
            .await
            .unwrap(),
    );
    let projects = Projects::new(storage.clone());
    let project = projects
        .create("p".into(), "p", "P", dir.path().to_str().unwrap())
        .await
        .unwrap()
        .id;
    let record = CodeRecord::new(storage.clone(), Codebase::new(storage.clone(), projects));
    let workflow = WorkflowId::from_recorded("workflow".into());
    let done = DoneRecord {
        project: project.clone(),
        workflow: workflow.clone(),
        version: 1,
        task: 1,
        attempt: None,
        commit: None,
        verdict: None,
        contract: serde_json::from_value(json!({
            "elements": [
                {"id": "E1", "revision": 1, "action": "create", "file": "users.sql"},
                {"id": "E2", "revision": 1, "action": "create", "file": "absent.sql"},
                {"id": "E3", "revision": 2, "action": "delete", "file": "removed.sql"},
                {"id": "E4", "revision": 1, "action": "create", "file": "users.sql",
                 "symbol_id": "s1"}
            ]
        }))
        .unwrap(),
    };
    storage
        .write_txn(move |tx| Box::pin(async move { CodeRecord::record_done_in(tx, &done).await }))
        .await
        .unwrap();

    let planned = PlannedPin {
        workflow_id: workflow,
        version: 1,
        task: 2,
        pin: ElementPin {
            id: "E1".into(),
            revision: 2,
            action: ElementAction::Modify,
            file: "users.sql".into(),
            symbol_id: None,
        },
    };
    let states = record
        .element_states(
            &project,
            &[
                element(&project, "E1", 2, false),
                element(&project, "E2", 1, false),
                element(&project, "E3", 2, true),
                element(&project, "E4", 1, false),
                element(&project, "E5", 1, false),
            ],
            &[planned],
        )
        .await
        .unwrap();
    assert_eq!(
        states
            .iter()
            .map(|element| element.state)
            .collect::<Vec<_>>(),
        [
            BuildState::Built,
            BuildState::Gap,
            BuildState::Removed,
            BuildState::Unchecked,
            BuildState::Planned
        ]
    );
    assert!(states[0].behind);
    assert_eq!(states[0].built_revision, Some(1));
    assert_eq!(states[0].confirmed, Some(Confirmation::FileOnly));
    assert_eq!(states[0].planned[0].revision, 2);
    assert_eq!(
        states[1].why.as_deref(),
        Some("`absent.sql` is absent; T1 of workflow v1 built it")
    );
    assert_eq!(
        states[3].why.as_deref(),
        Some("no language server confirmed s1 in users.sql")
    );
}

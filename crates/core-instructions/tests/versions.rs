use std::sync::Arc;

use core_db::{Storage, StorageError};
use core_instructions::{Instructions, InstructionsError};
use core_projects::{ProjectId, Projects};

struct World {
    storage: Arc<Storage>,
    instructions: Instructions,
    project: ProjectId,
    other: ProjectId,
    _tmp: tempfile::TempDir,
}

async fn world() -> World {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap());
    let projects = Projects::new(storage.clone());
    let mut ids = Vec::new();
    for slug in ["project", "other"] {
        ids.push(
            projects
                .create(slug.into(), slug, slug, tmp.path().to_str().unwrap())
                .await
                .unwrap()
                .id,
        );
    }
    World {
        instructions: Instructions::new(storage.clone()),
        storage,
        project: ids.remove(0),
        other: ids.remove(0),
        _tmp: tmp,
    }
}

#[tokio::test]
async fn saved_versions_replay_exactly_and_remain_scoped_to_the_project() {
    let w = world().await;
    assert_eq!(w.instructions.current(&w.project).await.unwrap(), None);
    let first = w
        .instructions
        .save("first".into(), &w.project, "one")
        .await
        .unwrap();
    let second = w
        .instructions
        .save("second".into(), &w.project, "two")
        .await
        .unwrap();
    assert_eq!((first.number, second.number), (1, 2));
    assert_eq!(
        w.instructions.current(&w.project).await.unwrap(),
        Some(second)
    );
    assert_eq!(w.instructions.current(&w.other).await.unwrap(), None);
    assert_eq!(
        w.instructions
            .save("first".into(), &w.project, "one")
            .await
            .unwrap(),
        first,
    );
    assert!(matches!(
        w.instructions
            .save("first".into(), &w.project, "changed")
            .await,
        Err(InstructionsError::Storage(StorageError::CommandConflict))
    ));
    let other = w
        .instructions
        .save("first".into(), &w.other, "other")
        .await
        .unwrap();
    assert_eq!(other.number, 1);
    let payloads: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM durable_event WHERE kind='PlannerInstructionsSaved' ORDER BY seq",
    )
    .fetch_all(w.storage.reader())
    .await
    .unwrap();
    assert_eq!(
        payloads,
        [r#"{"number":1}"#, r#"{"number":2}"#, r#"{"number":1}"#]
    );
}

#[tokio::test]
async fn concurrent_saves_number_versions_inside_the_write() {
    let w = world().await;
    let mut saves = tokio::task::JoinSet::new();
    for n in 0..12 {
        let service = w.instructions.clone();
        let project = w.project.clone();
        saves.spawn(async move {
            service
                .save(format!("save-{n}"), &project, "body")
                .await
                .unwrap()
                .number
        });
    }
    let mut numbers = Vec::new();
    while let Some(result) = saves.join_next().await {
        numbers.push(result.unwrap());
    }
    numbers.sort();
    assert_eq!(numbers, (1..=12).collect::<Vec<_>>());
    assert_eq!(
        w.instructions
            .current(&w.project)
            .await
            .unwrap()
            .unwrap()
            .number,
        12
    );
}

#[tokio::test]
async fn removal_hides_current_but_keeps_the_original_replay() {
    let w = world().await;
    let first = w
        .instructions
        .save("saved".into(), &w.project, "original")
        .await
        .unwrap();
    let project = w.project.clone();
    w.storage
        .write_txn(move |conn| {
            Box::pin(
                async move { Projects::mark_removed_in(conn, &project, &Storage::now()).await },
            )
        })
        .await
        .unwrap();
    assert_eq!(w.instructions.current(&w.project).await.unwrap(), None);
    assert_eq!(
        w.instructions
            .save("saved".into(), &w.project, "original")
            .await
            .unwrap(),
        first,
    );
    assert!(matches!(
        w.instructions.save("new".into(), &w.project, "new").await,
        Err(InstructionsError::Storage(StorageError::NotFound(
            "project"
        )))
    ));
}

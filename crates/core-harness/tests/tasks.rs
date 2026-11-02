mod fixture;

use core_threads::ThreadId;
use fixture::fixture;
use shadows_agent::events::TaskUpdate;

#[tokio::test]
async fn tasks_outlive_the_turn_that_started_them() {
    let (_tmp, storage, harness) = fixture().await;
    let thread = ThreadId::generate();
    let mut changes = harness.watch_tasks();
    harness.report_task_for_test(
        &thread,
        TaskUpdate::Spawned {
            async_task_id: "one".into(),
            name: "job".into(),
            task_type: "shell".into(),
            description: "run".into(),
            show_in_transcript: false,
            can_stop: true,
            output_file_path: None,
            tool_call_id: None,
        },
    );
    let (_, _, tasks) = changes.recv().await.unwrap();
    assert_eq!(tasks[0].state, "running");
    assert_eq!(harness.tasks_of(&thread), tasks);
    harness.report_task_for_test(
        &thread,
        TaskUpdate::State {
            async_task_id: "one".into(),
            state: "completed".into(),
            summary: Some("done".into()),
            output_file_path: None,
            tool_call_id: None,
        },
    );
    assert_eq!(
        harness.tasks_of(&thread)[0].summary.as_deref(),
        Some("done")
    );
    harness.close_session(&thread).await.unwrap();
    assert!(harness.tasks_of(&thread).is_empty());
    storage.close().await.unwrap();
}

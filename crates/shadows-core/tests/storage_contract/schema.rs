//! A fresh database: its connection policy and its tables.

use shadows_core::testing::Storage;

#[tokio::test]
async fn fresh_database_migrates_and_applies_the_connection_policy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("shadows.sqlite3");
    let storage = Storage::open(&db).await.expect("open should succeed");

    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(journal.to_lowercase(), "wal", "spec §6.23 requires WAL");

    let fk: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(storage.reader())
        .await
        .unwrap();
    assert_eq!(fk, 1, "spec §6.23 requires foreign_keys = ON");

    let mut tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' \
         AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(storage.reader())
    .await
    .unwrap();
    tables.sort();

    assert_eq!(
        tables,
        vec![
            "agent_invocation",
            "agreement",
            "agreement_operation_identity",
            "agreement_version",
            "code_file",
            "code_setting",
            "code_tag",
            "command_record",
            "design_command_result",
            "design_outcome",
            "design_outcome_part",
            "design_outcome_plan",
            "design_part",
            "design_part_plan",
            "design_workspace",
            "draft_intent",
            "durable_event",
            "harness_limit",
            "harness_model_effort",
            "harness_preference",
            "mcp_grant",
            "operation",
            "plan",
            "planner_instructions_version",
            "planning_thread",
            "project",
            "project_link",
            "project_mode",
            "queued_message",
            "runtime_instance",
            "task",
            "task_agreement_binding",
            "task_parent",
            "task_plan_parent",
            "thread_entry",
            "workflow",
        ],
        "spec §7.1: the migrations carry only the milestones' tables \
         (0005 adds §12's four, 0007 §13.15's six, 0008 §15.4's four, \
         0011 §12.4's per-model effort, 0012 §16.9's plan, 0013 §18's vision, \
         0017–0018 §18's agreements and exact task pins, \
         0019 §20's queue)"
    );
}

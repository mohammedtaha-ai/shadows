//! A fresh database: its connection policy and its tables.

use core_db::Storage;

#[tokio::test]
async fn fresh_database_migrates_and_applies_the_connection_policy() {
    // A folder name that reads as percent-encoding is opened as written: beside it, the
    // decoded name must stay empty.
    let tmp = tempfile::tempdir().unwrap();
    let (literal, decoded) = (tmp.path().join("team%20A"), tmp.path().join("team A"));
    std::fs::create_dir(&literal).unwrap();
    std::fs::create_dir(&decoded).unwrap();
    let db = literal.join("shadows.sqlite3");
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
            "attachment",
            "aylul_agent",
            "aylul_agent_extension",
            "aylul_agent_mcp",
            "aylul_agent_mcp_tool_rule",
            "aylul_extension",
            "aylul_extension_declined",
            "aylul_mcp_server",
            "aylul_person",
            "aylul_provider",
            "aylul_usage",
            "code_file",
            "code_inventory",
            "code_inventory_scan",
            "code_record_element",
            "code_record_file",
            "code_record_symbol",
            "code_record_unpromised",
            "code_record_verdict",
            "code_server_added",
            "code_server_off",
            "code_tag",
            "command_record",
            "context_forget",
            "design_command_result",
            "design_decisions",
            "design_elements",
            "design_map_proposal",
            "design_map_version",
            "design_outcome",
            "design_outcome_part",
            "design_outcome_plan",
            "design_part",
            "design_part_plan",
            "design_part_waiver",
            "design_roadmap_proposal",
            "design_thread_anchor",
            "design_vision_proposal",
            "design_vision_version",
            "design_workspace",
            "draft_intent",
            "durable_event",
            "external_connection",
            "external_connection_project",
            "harness_limit",
            "harness_model_effort",
            "harness_preference",
            "harness_thread_mode",
            "invocation_context",
            "mcp_grant",
            "message",
            "message_cursor",
            "message_delivery",
            "message_group",
            "message_group_member",
            "operation",
            "person_question",
            "plan",
            "planner_instructions_version",
            "planning_thread",
            "project",
            "project_extension",
            "project_link",
            "project_mcp",
            "project_mode",
            "queued_message",
            "run_agent_messages",
            "run_attempt_context",
            "run_attempts",
            "run_checks",
            "run_executor_saves",
            "run_lessons",
            "run_recheck_results",
            "run_rechecks",
            "run_task_executors",
            "run_task_marks",
            "runtime_instance",
            "secret_env",
            "secret_env_import",
            "standards_additions_version",
            "task",
            "task_agreement_binding",
            "task_parent",
            "task_plan_parent",
            "thread_attachment",
            "thread_entry",
            "workflow",
        ],
        "spec §7.1: the migrations carry only the milestones' tables \
         (0005 adds §12's four, 0007 §13.15's six, 0008 §15.4's four, \
         0011 §12.4's per-model effort, 0012 §16.9's plan, 0013 §18's vision, \
         0017–0018 §18's agreements and exact task pins, \
         0019 §20's queue, 0021 §23's standards additions, \
         0023 §23's map proposals, versions and waivers, \
         0026 §23.5's roadmap proposals, 0028 AYLUL's encrypted environment, \
         0033 the person, 0034 usage, 0035 the extension library and its per-agent switches, \
         0036 the remembered conversation mode, \
         0039 the MCP library, its per-agent switches and declined bundled names, \
         0040 the per-agent MCP tool rules, 0042 agent groups and their delivery journal, \
         0054 the project's file inventory, 0056 the lessons record, \
         0058 the language servers turned off and added, \
         0060 the person's questions, 0061 the task context an attempt received, \
         0070 the recheck batches and their results, 0071 the project's design decisions, \
         0073 the task's executor choice and its saves, \
         0074 the external connections and their project scopes, \
         0075 the code record's verdicts, 0076 the code record, \
         0080 the project's Planner switches)"
    );

    storage.close().await.unwrap();
    assert!(db.is_file(), "the database is at the path asked for");
    assert_eq!(
        std::fs::read_dir(&decoded).unwrap().count(),
        0,
        "nothing in the decoded path"
    );
}

#[test]
fn now_always_writes_nine_fraction_digits() {
    for _ in 0..2000 {
        let now = Storage::now();
        assert_eq!(
            now.len(),
            30,
            "fixed width so text order is time order: {now}"
        );
        assert!(now.ends_with('Z') && now.as_bytes()[19] == b'.', "{now}");
    }
}

#[test]
fn an_instant_reads_any_stored_width_in_time_order() {
    let at = |text| Storage::instant(text).unwrap();
    assert!(at("2026-11-25T12:00:00.1Z") < at("2026-11-25T12:00:00.12Z"));
    assert!(at("2026-11-25T12:00:00Z") < at("2026-11-25T12:00:00.5Z"));
    assert_eq!(
        at("2026-11-25T12:00:00.5Z"),
        at("2026-11-25T12:00:00.500000000Z")
    );
    assert!(Storage::instant("not a time").is_err());
}

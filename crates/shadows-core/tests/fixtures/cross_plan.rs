//! Synthetic project-owned plan versions for cross-plan tests.
use shadows_core::testing::{CommandContext, Storage};
use shadows_core::{AcceptanceItem, PlanOp, TaskContent};

pub const SOURCE: &str = "22222222-2222-4222-8222-222222222222";
pub const TARGET: &str = "33333333-3333-4333-8333-333333333333";
pub const TARGET_PLAN: &str = "44444444-4444-4444-8444-444444444444";

pub fn command(id: &str) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: id.into(),
        command_kind: "PlanEdit".into(),
        command_schema_ver: 1,
        request_fingerprint: id.into(),
    }
}

pub fn task(number: u32) -> PlanOp {
    PlanOp::TaskAdd {
        task: TaskContent {
            number,
            title: format!("Task {number}"),
            goal: "A usable result".into(),
            reads: vec![],
            writes: vec!["src/result.rs".into()],
            acceptance: vec![AcceptanceItem {
                number: 1,
                text: "The result works".into(),
            }],
        },
    }
}

pub async fn fixture() -> (tempfile::TempDir, Storage) {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&temp.path().join("links.sqlite3"))
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO project (id, slug, name, directory, created_at)
         VALUES ('p', 'p', 'Project', '.', '2026-10-04T00:00:00Z');
         INSERT INTO planning_thread (id, project_id, title, status, created_at)
         VALUES ('writer', 'p', 'Writer', 'Open', '2026-10-04T00:00:00Z');
         INSERT INTO plan (id, project_id, state, created_at)
         VALUES ('source-plan', 'p', 'Active', '2026-10-04T00:00:00Z'),
           ('44444444-4444-4444-8444-444444444444', 'p', 'Active', '2026-10-04T00:00:00Z');
         INSERT INTO workflow
           (id, plan_id, state, version, revision, title, goal, written_by_thread,
            created_at, updated_at)
         VALUES ('22222222-2222-4222-8222-222222222222', 'source-plan', 'Draft',
           1, 0, 'Web', 'A usable web app', 'writer',
           '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z'),
           ('33333333-3333-4333-8333-333333333333',
            '44444444-4444-4444-8444-444444444444', 'Draft', 1, 0,
            'Backend', 'A usable API', 'writer',
            '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z');",
    )
    .execute(storage.reader())
    .await
    .unwrap();
    (temp, storage)
}

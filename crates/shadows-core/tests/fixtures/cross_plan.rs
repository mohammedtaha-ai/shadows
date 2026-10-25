//! Synthetic project-owned plan versions for cross-plan tests.
use shadows_core::testing::{CommandContext, Storage};
use shadows_core::{AcceptanceItem, PlanOp, TaskContent};

pub const SOURCE: &str = "22222222-2222-4222-8222-222222222222";
pub const TARGET: &str = "33333333-3333-4333-8333-333333333333";
pub const TARGET_PLAN: &str = "44444444-4444-4444-8444-444444444444";

pub fn command(id: &str) -> CommandContext {
    CommandContext {
        agent: None,
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

pub async fn fixture() -> (tempfile::TempDir, std::sync::Arc<Storage>) {
    let temp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(
        Storage::open(&temp.path().join("links.sqlite3"))
            .await
            .unwrap(),
    );
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
    approve_roadmap(&storage, "p", &["source-plan"]).await;
    (temp, storage)
}

/// Puts `project` at the plans stage and ties each of `plans` to its seeded part.
pub async fn approve_roadmap(storage: &std::sync::Arc<Storage>, project: &str, plans: &[&str]) {
    let project = shadows_core::ProjectId::from_literal(project);
    let part = shadows_core::AppCore::design_for_tests(storage.clone())
        .seed_roadmap_for_tests(&project)
        .await
        .unwrap();
    for plan in plans {
        let plan = shadows_core::PlanId::from_literal(*plan);
        shadows_core::AppCore::plans_for_tests(storage.clone())
            .link_plan_part_for_test(&project, &plan, &part)
            .await
            .unwrap();
    }
}

//! The project's state: Design's elements, Plans' live versions, Runs' progress and
//! CodeRecord's element states, composed by ProjectState.
use super::*;
use shadows_core::{
    ElementContent, ElementInput, ElementSeed, Grant, PartId, PlanId, ScreenContent, TableColumn,
    TableContent,
};

/// A Planner turn on the plan's writer thread that stays running, so its element writes
/// are heard; answers that Planner's grant.
async fn planner(f: &Fixture) -> Grant {
    use shadows_core::testing::turn::{new_turn, turn_command};
    let grant = planner_grant(f).await;
    let thread = grant.thread_id.clone().unwrap();
    let runtime = shadows_core::testing::Runtime::start(f.storage.clone())
        .await
        .unwrap();
    let settings = shadows_agent::TurnSettings {
        model: "fake-large".into(),
        mode: "default".into(),
        effort: None,
    };
    let turn = new_turn(&thread, &runtime, "design", &settings);
    let started = shadows_core::Turns::start_turn_for_test(
        &f.storage,
        &turn_command("design", &thread, "design", &settings),
        turn,
    )
    .await
    .unwrap();
    shadows_core::Turns::mark_operation_started_for_test(
        &f.storage,
        &started.operation_id,
        runtime.instance_id(),
    )
    .await
    .unwrap();
    grant
}

/// The parts and outcome to write elements in, and a Planner that writes them.
async fn designed(f: &Fixture) -> (ElementSeed, Grant) {
    let seed = f
        .core
        .design()
        .seed_elements_for_tests(&f.project)
        .await
        .unwrap();
    (seed, planner(f).await)
}

fn users(seed: &ElementSeed) -> ElementInput {
    ElementInput {
        id: None,
        expected_revision: None,
        part_id: seed.database.clone(),
        outcomes: vec![seed.outcome.clone()],
        content: ElementContent::Table(TableContent {
            name: "users".into(),
            purpose: "Store users".into(),
            columns: vec![TableColumn {
                name: "id".into(),
                type_name: "TEXT".into(),
                meaning: String::new(),
                primary_key: true,
                references: None,
            }],
        }),
    }
}

fn sign_in(seed: &ElementSeed) -> ElementInput {
    ElementInput {
        id: None,
        expected_revision: None,
        part_id: seed.frontend.clone(),
        outcomes: vec![seed.outcome.clone()],
        content: ElementContent::Screen(ScreenContent {
            name: "Sign in".into(),
            purpose: "Let people sign in".into(),
            route: Some("/login".into()),
            leads_to: vec![],
        }),
    }
}

/// A new Active plan `id` with a Draft v1 written by the fixture's Planner, tied to `parts`.
async fn new_plan(f: &Fixture, id: &str, parts: &[&PartId]) -> WorkflowId {
    let thread: String = sqlx::query_scalar("SELECT written_by_thread FROM workflow WHERE id = ?")
        .bind(SOURCE)
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO plan (id, project_id, state, created_at)
         VALUES (?, ?, 'Active', '2026-10-06T00:00:00Z')",
    )
    .bind(id)
    .bind(f.project.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    let workflow = format!("{id}-v1");
    sqlx::query(
        "INSERT INTO workflow
           (id, plan_id, state, version, revision, title, goal, written_by_thread,
            created_at, updated_at)
         VALUES (?, ?, 'Draft', 1, 0, ?, 'People sign in', ?,
           '2026-10-06T00:00:00Z', '2026-10-06T00:00:00Z')",
    )
    .bind(&workflow)
    .bind(id)
    .bind(format!("Plan {id}"))
    .bind(&thread)
    .execute(f.storage.reader())
    .await
    .unwrap();
    let plans = AppCore::plans_for_tests(f.storage.clone());
    for part in parts {
        plans
            .link_plan_part_for_test(&f.project, &PlanId::from_literal(id), part)
            .await
            .unwrap();
    }
    WorkflowId::from_literal(workflow)
}

/// A task at `number` working `file` and pinning `element` r`revision` there.
fn pinning(number: u32, file: &str, element: &str, revision: i64) -> PlanOp {
    let contract = serde_json::from_value(serde_json::json!({
        "files": [{"path": file, "action": "create", "purpose": "Build it"}],
        "elements": [{"id": element, "revision": revision, "action": "create", "file": file}]
    }))
    .unwrap();
    let folder = file.split('/').next().unwrap();
    PlanOp::TaskAdd {
        task: TaskContent {
            phases: Vec::new(),
            contract: Some(contract),
            number,
            title: format!("Build {element}"),
            goal: "A usable result".into(),
            reads: vec![],
            writes: vec![format!("{folder}/**")],
            acceptance: vec![AcceptanceItem {
                number: 1,
                text: "It is built".into(),
            }],
        },
    }
}

async fn edit(f: &Fixture, key: &str, workflow: &WorkflowId, ops: &[PlanOp]) {
    AppCore::plans_for_tests(f.storage.clone())
        .edit_plan_for_test(&command(key), &Writer::Person, None, workflow, 0, ops)
        .await
        .unwrap();
}

async fn approve(f: &Fixture, key: &str, workflow: &WorkflowId) {
    AppCore::plans_for_tests(f.storage.clone())
        .approve_plan_for_test(&command(key), workflow, 1)
        .await
        .unwrap();
}

/// E1 (a table) and E2 (a screen); one plan in both parts whose T1 pins E1 in
/// `db/0001_users.sql` and T2 pins E2; approved, and the table's file written.
async fn built(f: &Fixture) -> (ElementSeed, Grant, WorkflowId) {
    let (seed, grant) = designed(f).await;
    let design = f.core.design();
    design
        .put_element_for(&grant, "users".into(), users(&seed))
        .await
        .unwrap();
    design
        .put_element_for(&grant, "sign-in".into(), sign_in(&seed))
        .await
        .unwrap();
    let workflow = new_plan(f, "state-plan", &[&seed.database, &seed.frontend]).await;
    let ops = [
        pinning(1, "db/0001_users.sql", "E1", 1),
        pinning(2, "web/login.tsx", "E2", 1),
    ];
    edit(f, "state-tasks", &workflow, &ops).await;
    approve(f, "state-approve", &workflow).await;
    f.put(
        "db/0001_users.sql",
        "CREATE TABLE users (id TEXT PRIMARY KEY);\n",
    );
    (seed, grant, workflow)
}

fn part<'a>(view: &'a shadows_core::ProjectStateView, id: &PartId) -> &'a shadows_core::PartState {
    view.parts.iter().find(|p| &p.part_id == id).unwrap()
}

#[tokio::test]
async fn project_state_counts_each_parts_plans_tasks_and_elements() {
    let f = fixture_with(false, None).await;
    let (seed, _, workflow) = built(&f).await;
    f.core
        .runs()
        .mark_done("done-1".into(), &workflow, 1, 1, true, None)
        .await
        .unwrap();
    let state = f.core.project_state();
    let view = state.state(&f.project, None).await.unwrap();
    let database = part(&view, &seed.database);
    assert_eq!(database.elements.built, 1, "{view:?}");
    assert_eq!(database.elements.planned, 0);
    let frontend = part(&view, &seed.frontend);
    assert_eq!(frontend.elements.planned, 1, "{view:?}");
    let plan = &database.plans[0];
    assert_eq!(plan.workflow_id.as_ref(), Some(&workflow));
    assert_eq!(plan.version, Some(1));
    assert_eq!(
        (plan.tasks.total, plan.tasks.done, plan.tasks.not_started),
        (2, 1, 1)
    );
    assert_eq!(plan.tasks.started, 0);
    assert_eq!(frontend.plans[0].plan_id, plan.plan_id);
    let elements = state.elements(&f.project).await.unwrap();
    let e1 = &elements[0].state;
    assert_eq!(e1.state, shadows_core::BuildState::Built);
    assert_eq!(e1.confirmed, Some(shadows_core::Confirmation::FileOnly));
    assert!(view.gaps.is_empty());
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_draft_adds_nothing_to_the_state() {
    let f = fixture_with(false, None).await;
    let (seed, grant) = designed(&f).await;
    f.core
        .design()
        .put_element_for(&grant, "users".into(), users(&seed))
        .await
        .unwrap();
    let workflow = new_plan(&f, "draft-plan", &[&seed.database]).await;
    edit(
        &f,
        "draft-tasks",
        &workflow,
        &[pinning(1, "db/0001_users.sql", "E1", 1)],
    )
    .await;
    let state = f.core.project_state();
    let view = state.state(&f.project, None).await.unwrap();
    let plan = &part(&view, &seed.database).plans[0];
    assert_eq!(plan.workflow_id, None);
    assert_eq!(plan.version, None);
    assert_eq!(plan.draft.as_ref(), Some(&workflow));
    assert_eq!(plan.tasks.total, 0);
    let elements = state.elements(&f.project).await.unwrap();
    assert!(elements[0].state.planned.is_empty(), "{elements:?}");
    assert_eq!(elements[0].state.state, shadows_core::BuildState::Planned);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn an_absent_file_is_a_gap_in_the_state() {
    let f = fixture_with(false, None).await;
    let (seed, _, workflow) = built(&f).await;
    f.core
        .runs()
        .mark_done("done-1".into(), &workflow, 1, 1, true, None)
        .await
        .unwrap();
    std::fs::remove_file(f.root.join("db/0001_users.sql")).unwrap();
    let view = f
        .core
        .project_state()
        .state(&f.project, None)
        .await
        .unwrap();
    assert_eq!(view.gaps.len(), 1, "{view:?}");
    let gap = &view.gaps[0];
    assert_eq!(gap.element_id, "E1");
    assert_eq!(gap.name, "users");
    assert_eq!(gap.part_id, seed.database);
    assert_eq!(gap.state, shadows_core::BuildState::Gap);
    assert_eq!(
        gap.why,
        format!("`db/0001_users.sql` is absent; T1 of {workflow} v1 built it")
    );
    assert_eq!(part(&view, &seed.database).elements.gap, 1);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_newer_revision_is_behind() {
    let f = fixture_with(false, None).await;
    let (seed, grant, workflow) = built(&f).await;
    f.core
        .runs()
        .mark_done("done-1".into(), &workflow, 1, 1, true, None)
        .await
        .unwrap();
    let mut next = users(&seed);
    next.id = Some("E1".into());
    next.expected_revision = Some(1);
    f.core
        .design()
        .put_element_for(&grant, "users-2".into(), next)
        .await
        .unwrap();
    let elements = f.core.project_state().elements(&f.project).await.unwrap();
    assert_eq!(elements[0].element.revision, 2);
    assert_eq!(elements[0].state.state, shadows_core::BuildState::Built);
    assert_eq!(elements[0].state.built_revision, Some(1));
    assert!(elements[0].state.behind);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_part_asked_in_full_lists_its_plans_tasks_and_elements() {
    let f = fixture_with(false, None).await;
    let (seed, _, workflow) = built(&f).await;
    f.core
        .runs()
        .mark_done("done-1".into(), &workflow, 1, 1, true, None)
        .await
        .unwrap();
    let state = f.core.project_state();
    let whole = state.state(&f.project, None).await.unwrap();
    assert!(whole.part.is_none());
    let view = state.state(&f.project, Some(&seed.database)).await.unwrap();
    let detail = view.part.unwrap();
    assert_eq!(detail.part_id, seed.database);
    assert_eq!(detail.plans.len(), 1);
    assert_eq!(detail.plans_more, 0);
    let plan = &detail.plans[0];
    assert_eq!((&plan.workflow_id, plan.version), (&workflow, 1));
    let lines: Vec<_> = plan
        .tasks
        .iter()
        .map(|t| (t.number, t.done, t.attempt))
        .collect();
    assert_eq!(lines, [(1, true, None), (2, false, None)]);
    assert_eq!(plan.tasks[0].title, "Build E1");
    let ids: Vec<_> = detail
        .elements
        .iter()
        .map(|e| e.element.id.as_str())
        .collect();
    assert_eq!(ids, ["E1"]);
    assert_eq!(
        detail.elements[0].state.state,
        shadows_core::BuildState::Built
    );
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn the_state_is_bounded() {
    let f = fixture_with(false, None).await;
    let (seed, _) = designed(&f).await;
    for n in 0..21 {
        new_plan(&f, &format!("bound-{n:02}"), &[&seed.database]).await;
    }
    let view = f
        .core
        .project_state()
        .state(&f.project, None)
        .await
        .unwrap();
    let database = part(&view, &seed.database);
    assert_eq!(database.plans.len(), 20);
    assert_eq!(database.plans_more, 1);
    assert_eq!(database.plans[0].plan_id.as_str(), "bound-00");
    assert_eq!(view.parts_more, 0);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn an_unknown_part_is_refused() {
    let f = fixture_with(false, None).await;
    let refused = f
        .core
        .project_state()
        .state(&f.project, Some(&PartId::from_literal("nope")))
        .await;
    let Err(shadows_core::ProjectStateError::UnknownPart(message)) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(message, "part nope is not a part of this project");
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_parts_plans_in_the_state_are_its_design_part_plans() {
    let f = fixture_with(false, None).await;
    let (seed, _) = designed(&f).await;
    for id in ["agree-a", "agree-b", "agree-c"] {
        new_plan(&f, id, &[&seed.database]).await;
    }
    sqlx::query(
        "UPDATE plan SET state = 'Archived', archived_at = '2026-10-07T00:00:00Z'
         WHERE id = 'agree-b'",
    )
    .execute(f.storage.reader())
    .await
    .unwrap();
    let view = f
        .core
        .project_state()
        .state(&f.project, None)
        .await
        .unwrap();
    let mut in_state: Vec<PlanId> = part(&view, &seed.database)
        .plans
        .iter()
        .map(|p| p.plan_id.clone())
        .collect();
    let active: Vec<PlanId> = (f.core.plans().list(&f.project, false).await.unwrap())
        .into_iter()
        .map(|p| p.plan_id)
        .collect();
    let mut of_part: Vec<PlanId> = (f.core.plans())
        .design_part(&f.project, &seed.database)
        .await
        .unwrap()
        .plans
        .into_iter()
        .filter(|p| active.contains(p))
        .collect();
    in_state.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    of_part.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(in_state, of_part);
    assert_eq!(in_state.len(), 2);
    f.storage.close().await.unwrap();
}

#[tokio::test]
async fn a_running_attempt_shows_started_with_its_state() {
    let f = fixture_with(false, None).await;
    let (seed, _, workflow) = built(&f).await;
    // The task builds its own file: none waits uncommitted in its writes.
    std::fs::remove_file(f.root.join("db/0001_users.sql")).unwrap();
    let runs = f.core.runs();
    runs.choose_executor_for_test(&workflow, 1, "claude-code", "fake-large", Some("high"))
        .await
        .unwrap();
    f.hold();
    let attempt = runs.start("start-1".into(), &workflow, 1, 1).await.unwrap();
    let state = f.core.project_state();
    let line = |view: &shadows_core::ProjectStateView| {
        let plan = &view.part.as_ref().unwrap().plans[0];
        (plan.tasks[0].done, plan.tasks[0].attempt)
    };
    let view = state.state(&f.project, Some(&seed.database)).await.unwrap();
    assert_eq!(line(&view), (false, Some(AttemptState::Running)));
    let counts = part(&view, &seed.database).plans[0].tasks;
    assert_eq!((counts.started, counts.done), (1, 0));
    f.release();
    assert_eq!(f.settled(&attempt).await.state, AttemptState::Failed);
    let view = state.state(&f.project, Some(&seed.database)).await.unwrap();
    assert_eq!(line(&view), (false, Some(AttemptState::Failed)));
    let counts = part(&view, &seed.database).plans[0].tasks;
    assert_eq!((counts.started, counts.done), (1, 0));
    f.finish().await;
}

#[tokio::test]
async fn live_versions_name_the_highest_frozen_version_and_skip_drafts() {
    let f = fixture_with(false, None).await;
    let (seed, _, workflow) = built(&f).await;
    sqlx::query(
        "INSERT INTO workflow (id,plan_id,state,version,revision,title,goal,
         written_by_thread,previous_version_id,created_at,updated_at)
         SELECT 'state-plan-v2',plan_id,'Draft',2,0,'Changed','Changed',
         written_by_thread,id,created_at,updated_at FROM workflow WHERE id=?",
    )
    .bind(workflow.as_str())
    .execute(f.storage.reader())
    .await
    .unwrap();
    let draft_only = new_plan(&f, "zz-draft", &[]).await;
    let archived = new_plan(&f, "zz-archived", &[]).await;
    sqlx::query(
        "UPDATE plan SET state = 'Archived', archived_at = '2026-10-07T00:00:00Z'
         WHERE id = 'zz-archived'",
    )
    .execute(f.storage.reader())
    .await
    .unwrap();
    let live = f.core.plans().live_versions(&f.project).await.unwrap();
    assert!(!live.iter().any(|v| v.draft.as_ref() == Some(&archived)));
    let plan = live
        .iter()
        .find(|v| v.plan_id.as_str() == "state-plan")
        .unwrap();
    let frozen = plan.live.as_ref().unwrap();
    assert_eq!((&frozen.workflow_id, frozen.version), (&workflow, 1));
    assert_eq!(
        plan.draft.as_ref().map(|d| d.as_str()),
        Some("state-plan-v2")
    );
    assert_eq!(plan.title, "Plan state-plan");
    let mut parts = vec![seed.database.clone(), seed.frontend.clone()];
    parts.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(plan.parts, parts);
    let pins: Vec<_> = frozen
        .tasks
        .iter()
        .map(|t| (t.number, t.elements[0].id.as_str()))
        .collect();
    assert_eq!(pins, [(1, "E1"), (2, "E2")]);
    let drafted = live
        .iter()
        .find(|v| v.plan_id.as_str() == "zz-draft")
        .unwrap();
    assert!(drafted.live.is_none());
    assert_eq!(drafted.draft.as_ref(), Some(&draft_only));
    // The source fixture plan is listed first: list's order, by creation then id.
    let order: Vec<_> = live.iter().map(|v| v.plan_id.as_str()).collect();
    let listed: Vec<String> = (f.core.plans().list(&f.project, false).await.unwrap())
        .into_iter()
        .map(|p| p.plan_id.as_str().to_string())
        .collect();
    assert_eq!(order, listed);
    f.storage.close().await.unwrap();
}

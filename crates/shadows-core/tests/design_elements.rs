//! Table and screen revisions written through Design by a running Planner.

use std::sync::Arc;
use std::time::Duration;

use shadows_core::testing::{LiveHandles, Storage, acp};
use shadows_core::{
    AppCore, ColumnRef, CoreError, CoreParts, DesignElement, DesignError, ElementContent,
    ElementDelete, ElementInput, ElementSeed, ErrorCode, Grant, GrantKind, OperationId, ProjectId,
    ScreenContent, ScreenLink, SendTurn, StorageError, TableColumn, TableContent,
};

struct Fixture {
    core: Arc<AppCore>,
    storage: Arc<Storage>,
    project: ProjectId,
    grant: Grant,
    turn: OperationId,
    seed: ElementSeed,
    tmp: tempfile::TempDir,
}

impl Fixture {
    async fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let storage = Arc::new(
            Storage::open(&tmp.path().join("design.sqlite3"))
                .await
                .unwrap(),
        );
        let runtime = Arc::new(AppCore::runtime_for_tests(storage.clone()).await.unwrap().0);
        let core = AppCore::assemble(CoreParts {
            operations: AppCore::operations_for_tests(storage.clone()),
            secrets: shadows_core::testing::secrets(storage.clone(), None),
            storage: storage.clone(),
            runtime,
            sessions: acp::fake_sessions(storage.clone()),
            handles: Arc::new(LiveHandles::default()),
            bus: tokio::sync::broadcast::channel(32).0,
            ui: tokio::sync::broadcast::channel(32).0,
            mcp_url: acp::MCP_URL.into(),
        });
        let project = core
            .project_lifecycle()
            .create(
                "project".into(),
                "elements",
                "Elements",
                tmp.path().to_str().unwrap(),
            )
            .await
            .unwrap()
            .id;
        let seed = core
            .design()
            .seed_elements_for_tests(&project)
            .await
            .unwrap();
        let thread = core
            .threads()
            .create("thread".into(), &project, "Planner", None, None)
            .await
            .unwrap();
        let turn = core
            .turns()
            .send(
                thread.id.clone(),
                SendTurn {
                    attachments: vec![],
                    command_id: "turn".into(),
                    prompt: "hang".into(),
                    model: "fake-large".into(),
                    mode: "acceptEdits".into(),
                    effort: Some("high".into()),
                    focus: None,
                    plan: None,
                    client_tab: None,
                },
            )
            .await
            .unwrap();
        let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&storage, &thread.id)
            .await
            .unwrap();
        Self {
            core,
            storage,
            project,
            grant,
            turn,
            seed,
            tmp,
        }
    }

    fn table(&self, name: &str) -> ElementInput {
        ElementInput {
            id: None,
            expected_revision: None,
            part_id: self.seed.database.clone(),
            outcomes: vec![self.seed.outcome.clone()],
            content: ElementContent::Table(TableContent {
                name: name.into(),
                purpose: "Store project information".into(),
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

    fn screen(&self, name: &str, route: &str) -> ElementInput {
        ElementInput {
            id: None,
            expected_revision: None,
            part_id: self.seed.frontend.clone(),
            outcomes: vec![self.seed.outcome.clone()],
            content: ElementContent::Screen(ScreenContent {
                name: name.into(),
                purpose: "Show information".into(),
                route: Some(route.into()),
                leads_to: vec![],
            }),
        }
    }

    async fn put(&self, cmd: &str, input: ElementInput) -> Result<DesignElement, DesignError> {
        self.core
            .design()
            .put_element_for(&self.grant, cmd.into(), input)
            .await
    }

    async fn delete(&self, cmd: &str, id: &str, rev: i64) -> Result<DesignElement, DesignError> {
        self.core
            .design()
            .delete_element_for(
                &self.grant,
                cmd.into(),
                ElementDelete {
                    id: id.into(),
                    expected_revision: rev,
                },
            )
            .await
    }

    async fn list(&self) -> Vec<DesignElement> {
        self.core.design().elements(&self.project).await.unwrap()
    }

    async fn revisions(&self, id: &str) -> Vec<DesignElement> {
        self.core
            .design()
            .element_revisions(&self.project, id)
            .await
            .unwrap()
    }

    async fn count(&self, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(self.storage.reader())
            .await
            .unwrap()
    }

    async fn close(self) {
        self.core
            .shut_down(Duration::from_secs(3), std::future::pending())
            .await
            .unwrap();
        self.storage.close().await.unwrap();
    }
}

fn next(input: ElementInput, id: &str, expected: i64) -> ElementInput {
    ElementInput {
        id: Some(id.into()),
        expected_revision: Some(expected),
        ..input
    }
}

fn table_ref(input: &mut ElementInput, table_id: &str, column: &str) {
    let ElementContent::Table(table) = &mut input.content else {
        panic!("expected a table");
    };
    table.columns.push(TableColumn {
        name: "parent_id".into(),
        type_name: "TEXT".into(),
        meaning: String::new(),
        primary_key: false,
        references: Some(ColumnRef {
            table: table_id.into(),
            column: column.into(),
        }),
    });
}

fn screen_link(input: &mut ElementInput, target: &str) {
    let ElementContent::Screen(screen) = &mut input.content else {
        panic!("expected a screen");
    };
    screen.leads_to.push(ScreenLink {
        screen: target.into(),
        when: "After sign-in".into(),
    });
}

fn invalid(result: Result<DesignElement, DesignError>, expected: &str) {
    let error = result.unwrap_err();
    assert!(matches!(error, DesignError::InvalidElement(_)), "{error:?}");
    assert_eq!(error.to_string(), expected);
}

#[tokio::test]
async fn the_planner_writes_a_table_and_a_new_revision_keeps_its_id() {
    let f = Fixture::new().await;
    let seed = f
        .core
        .design()
        .seed_elements_for_tests(&f.project)
        .await
        .unwrap();
    assert_eq!(seed.database, f.seed.database);
    assert_eq!(seed.frontend, f.seed.frontend);
    assert_eq!(seed.outcome, f.seed.outcome);
    let first = f.put("first", f.table("users")).await.unwrap();
    let second = f
        .put("second", next(f.table("people"), "E1", 1))
        .await
        .unwrap();
    assert_eq!((first.id.as_str(), first.revision), ("E1", 1));
    assert_eq!((second.id.as_str(), second.revision), ("E1", 2));
    assert_eq!(f.revisions("E1").await, vec![first.clone(), second.clone()]);
    assert_eq!(f.list().await, vec![second.clone()]);
    let mut tx = f.storage.reader().begin().await.unwrap();
    for (revision, expected) in [(None, second), (Some(1), first)] {
        let found = shadows_core::Design::element_in(&mut tx, &f.project, "E1", revision)
            .await
            .unwrap();
        assert_eq!(found, Some(expected));
    }
    tx.commit().await.unwrap();
    assert!(f.revisions("E2").await.is_empty());
    f.core
        .turns()
        .remove_thread("remove-thread".into(), f.grant.thread_id.as_ref().unwrap())
        .await
        .unwrap();
    f.core
        .project_lifecycle()
        .remove("remove-project".into(), &f.project)
        .await
        .unwrap();
    let design = f.core.design();
    assert!(design.elements(&f.project).await.is_err());
    assert!(design.element_revisions(&f.project, "E1").await.is_err());
    assert_eq!(f.count("SELECT count(*) FROM design_elements").await, 2);
    f.close().await;
}

#[tokio::test]
async fn a_replayed_put_returns_the_first_revision() {
    let f = Fixture::new().await;
    let first = f.put("first", f.table("users")).await.unwrap();
    f.put("second", next(f.table("people"), "E1", 1))
        .await
        .unwrap();
    assert_eq!(f.put("first", f.table("users")).await.unwrap(), first);
    let conflict = f.put("first", f.table("accounts")).await.unwrap_err();
    assert!(matches!(
        conflict,
        DesignError::Storage(StorageError::CommandConflict)
    ));
    assert_eq!(f.revisions("E1").await.len(), 2);
    assert_eq!(
        f.count("SELECT count(*) FROM durable_event WHERE kind='DesignElementChanged'")
            .await,
        2
    );
    f.close().await;
}

#[tokio::test]
async fn a_stale_revision_is_refused() {
    let f = Fixture::new().await;
    f.put("first", f.table("users")).await.unwrap();
    f.put("second", next(f.table("people"), "E1", 1))
        .await
        .unwrap();
    let error = f
        .put("stale", next(f.table("accounts"), "E1", 1))
        .await
        .unwrap_err();
    assert!(matches!(error, DesignError::ElementStale(_)), "{error:?}");
    assert_eq!(error.to_string(), "E1 is at revision 2, not 1");
    assert_eq!(f.revisions("E1").await.len(), 2);
    f.close().await;
}

#[tokio::test]
async fn a_table_needs_a_database_part_and_a_screen_a_frontend_part() {
    let f = Fixture::new().await;
    let mut table = f.table("users");
    table.part_id = f.seed.frontend.clone();
    invalid(
        f.put("table", table).await,
        &format!(
            "a table belongs to a part of kind database; {} is frontend",
            f.seed.frontend
        ),
    );
    let mut screen = f.screen("Login", "/login");
    screen.part_id = f.seed.database.clone();
    invalid(
        f.put("screen", screen).await,
        &format!(
            "a screen belongs to a part of kind frontend; {} is database",
            f.seed.database
        ),
    );
    assert!(f.list().await.is_empty());
    f.close().await;
}

#[tokio::test]
async fn an_element_serves_an_outcome_of_the_project() {
    let f = Fixture::new().await;
    let mut table = f.table("users");
    table.outcomes.clear();
    invalid(
        f.put("empty", table.clone()).await,
        "an element serves at least one roadmap outcome",
    );
    let dir = f.tmp.path().join("foreign");
    std::fs::create_dir(&dir).unwrap();
    let foreign = f
        .core
        .project_lifecycle()
        .create(
            "foreign".into(),
            "foreign",
            "Foreign",
            dir.to_str().unwrap(),
        )
        .await
        .unwrap();
    let seed = f
        .core
        .design()
        .seed_elements_for_tests(&foreign.id)
        .await
        .unwrap();
    let message = format!("outcome {} is not in this project's roadmap", seed.outcome);
    table.outcomes.push(seed.outcome);
    invalid(f.put("foreign", table).await, &message);
    assert!(f.list().await.is_empty());
    f.close().await;
}

#[tokio::test]
async fn a_second_table_with_the_same_name_is_refused() {
    let f = Fixture::new().await;
    f.put("first", f.table("users")).await.unwrap();
    invalid(
        f.put("duplicate", f.table("Users")).await,
        "a table named `Users` already exists: E1",
    );
    assert_eq!(
        f.put("screen", f.screen("users", "/users"))
            .await
            .unwrap()
            .id,
        "E2"
    );
    f.close().await;
}

#[tokio::test]
async fn a_foreign_key_names_a_live_table_and_its_column() {
    let f = Fixture::new().await;
    f.put("screen", f.screen("Home", "/")).await.unwrap();
    f.put("target", f.table("users")).await.unwrap();
    let mut to_screen = f.table("accounts");
    table_ref(&mut to_screen, "E1", "id");
    invalid(
        f.put("to-screen", to_screen).await,
        "`parent_id` references E1, which is not a table of this project",
    );
    let mut to_missing_column = f.table("accounts");
    table_ref(&mut to_missing_column, "E2", "email");
    invalid(
        f.put("to-col", to_missing_column).await,
        "`parent_id` references E2.email, which E2 does not have",
    );
    f.delete("delete", "E2", 1).await.unwrap();
    let mut to_deleted = f.table("accounts");
    table_ref(&mut to_deleted, "E2", "id");
    invalid(
        f.put("to-deleted", to_deleted).await,
        "`parent_id` references E2, which is not a table of this project",
    );
    let mut self_ref = f.table("accounts");
    table_ref(&mut self_ref, "E3", "id");
    let first = f.put("self-ref", self_ref).await.unwrap();
    assert_eq!(first.id, "E3");
    assert!(f.delete("self-delete", "E3", 1).await.unwrap().deleted);
    f.close().await;
}

#[tokio::test]
async fn a_referenced_column_cannot_be_dropped_and_a_referenced_table_cannot_be_deleted() {
    let f = Fixture::new().await;
    f.put("target", f.table("users")).await.unwrap();
    let mut accounts = f.table("accounts");
    table_ref(&mut accounts, "E1", "id");
    f.put("referrer", accounts).await.unwrap();
    let mut input = next(f.table("users"), "E1", 1);
    let ElementContent::Table(users) = &mut input.content else {
        panic!("expected a table");
    };
    users.columns[0].name = "user_id".into();
    invalid(
        f.put("rename", input).await,
        "users.id is referenced by accounts.parent_id (E2); change E2 first",
    );
    invalid(
        f.delete("delete", "E1", 1).await,
        "E1 is referenced by E2 (accounts.parent_id); change E2 first",
    );
    f.put("unlink", next(f.table("accounts"), "E2", 1))
        .await
        .unwrap();
    assert!(f.delete("delete", "E1", 1).await.unwrap().deleted);
    f.close().await;
}

#[tokio::test]
async fn a_screen_leads_only_to_screens_and_its_route_is_unique() {
    let f = Fixture::new().await;
    f.put("table", f.table("users")).await.unwrap();
    f.put("home", f.screen("Home", "/")).await.unwrap();
    let mut login = f.screen("Login", "/login");
    screen_link(&mut login, "E1");
    invalid(
        f.put("to-table", login).await,
        "E1 is not a screen of this project",
    );
    invalid(
        f.put("same-route", f.screen("Login", "/")).await,
        "the route `/` is already E2's",
    );
    let mut login = f.screen("Login", "/login");
    screen_link(&mut login, "E2");
    assert_eq!(f.put("login", login).await.unwrap().id, "E3");
    invalid(
        f.delete("delete", "E2", 1).await,
        "E2 is referenced by E3 (leads to it); change E3 first",
    );
    f.close().await;
}

#[tokio::test]
async fn a_deleted_element_takes_no_new_revision() {
    let f = Fixture::new().await;
    let first = f.put("first", f.table("users")).await.unwrap();
    let deleted = f.delete("delete", "E1", 1).await.unwrap();
    assert_eq!(
        (deleted.id.as_str(), deleted.revision, deleted.deleted),
        ("E1", 2, true)
    );
    assert_eq!(f.delete("delete", "E1", 1).await.unwrap(), deleted);
    assert_eq!(f.put("first", f.table("users")).await.unwrap(), first);
    invalid(
        f.put("restore", next(f.table("users"), "E1", 2)).await,
        "E1 is deleted; write a new element",
    );
    assert_eq!(f.list().await, vec![deleted]);
    f.close().await;
}

#[tokio::test]
async fn ids_are_never_reused() {
    let f = Fixture::new().await;
    assert_eq!(f.put("first", f.table("users")).await.unwrap().id, "E1");
    assert_eq!(f.put("second", f.table("accounts")).await.unwrap().id, "E2");
    f.delete("delete", "E2", 1).await.unwrap();
    assert_eq!(f.put("third", f.table("accounts")).await.unwrap().id, "E3");
    let ids: Vec<_> = f
        .list()
        .await
        .into_iter()
        .map(|element| element.id)
        .collect();
    assert_eq!(ids, ["E1", "E2", "E3"]);
    f.close().await;
}

#[tokio::test]
async fn only_a_planners_running_turn_writes() {
    let f = Fixture::new().await;
    let mut executor = f.grant.clone();
    executor.kind = GrantKind::Executor;
    let error = f
        .core
        .design()
        .put_element_for(&executor, "write".into(), f.table("users"))
        .await
        .unwrap_err();
    assert_eq!(CoreError::from(error).code(), ErrorCode::GrantScope);
    f.core
        .turns()
        .stop_running(f.grant.thread_id.as_ref().unwrap())
        .await
        .unwrap();
    let error = f.put("write", f.table("users")).await.unwrap_err();
    assert!(matches!(error, DesignError::InvalidCommand(_)), "{error:?}");
    assert_eq!(
        error.to_string(),
        "no turn is running for this conversation"
    );
    assert!(f.list().await.is_empty());
    f.close().await;
}

#[tokio::test]
async fn the_write_journals_design_element_changed() {
    let f = Fixture::new().await;
    let first = f.put("first", f.table("users")).await.unwrap();
    let row: (String, String, String, String, String, String) = sqlx::query_as(
        "SELECT project_id, thread_id, operation_id, actor_kind, actor_id, payload_json
           FROM durable_event WHERE kind = 'DesignElementChanged'",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(row.0, f.project.as_str());
    assert_eq!(row.1, f.grant.thread_id.as_ref().unwrap().as_str());
    assert_eq!(row.2, f.turn.as_str());
    assert_eq!(row.3, "Thread");
    assert_eq!(row.4, row.1);
    assert_eq!(first.written_by, row.1);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&row.5).unwrap(),
        serde_json::json!({"project_id": f.project, "element_id": "E1", "revision": 1})
    );
    f.close().await;
}

#[tokio::test]
async fn the_project_has_at_most_500_elements() {
    let f = Fixture::new().await;
    for n in 1..=500 {
        let element = f
            .put(&format!("put-{n}"), f.table(&format!("table-{n}")))
            .await
            .unwrap();
        assert_eq!(element.id, format!("E{n}"));
    }
    invalid(
        f.put("overflow", f.table("overflow")).await,
        "this project has 500 tables and screens; delete one first",
    );
    let revision = f
        .put("revision", next(f.table("first"), "E1", 1))
        .await
        .unwrap();
    assert_eq!(revision.revision, 2);
    let listed = f.list().await;
    assert_eq!(listed.len(), 500);
    assert_eq!(listed[9].id, "E10");
    f.delete("delete", "E2", 1).await.unwrap();
    assert_eq!(
        f.put("overflow", f.table("overflow")).await.unwrap().id,
        "E501"
    );
    f.close().await;
}

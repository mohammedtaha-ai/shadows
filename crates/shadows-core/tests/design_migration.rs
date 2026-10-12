//! Preservation across 0013–0015 on a populated pre-workspace database.
use std::{borrow::Cow, str::FromStr};

use shadows_core::testing::{CommandContext, Storage};
use shadows_core::{
    DesignAnchor, DesignOp, OutcomeContent, OutcomeId, PartContent, PartId, PlanId, ProjectId,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

const PROJECT: &str = "00000000-0000-4000-8000-000000000001";
const PLAN: &str = "00000000-0000-4000-8000-000000000002";

#[tokio::test]
async fn migration_24_backfills_preexisting_approved_proposal_views() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("pre24.sqlite3");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let mut old = sqlx::migrate!("./migrations");
    old.migrations = Cow::Owned(
        old.migrations
            .iter()
            .filter(|m| m.version <= 23)
            .cloned()
            .collect(),
    );
    old.run(&pool).await.unwrap();
    sqlx::raw_sql(
        "INSERT INTO project(id,slug,name,directory,created_at)
         VALUES ('P','old','Old','C:/old','old');
         INSERT INTO planning_thread(id,project_id,title,status,created_at,title_source)
         VALUES ('T','P','Old','Open','old','client');
         INSERT INTO runtime_instance(id,version,started_at) VALUES ('R','old','old');
         INSERT INTO operation(id,kind,thread_id,runtime_instance_id,created_at)
         VALUES ('O','PlannerTurn','T','R','old');
         INSERT INTO thread_entry(id,thread_id,ordinal,kind,author_kind,author_id,body,created_at)
         VALUES ('E','T',1,'VisionProposal','Thread','T','Old','old');",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO design_workspace(project_id,revision,vision_revision,vision_content)
         VALUES ('P',0,0,?)",
    )
    .bind(serde_json::to_string(&shadows_core::VisionContent::default()).unwrap())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO design_vision_proposal
         (id,project_id,thread_id,operation_id,entry_id,action,field,text,status,
          standards_version,created_at,seq)
         VALUES ('V','P','T','O','E','add','goals','Old','Approved',1,'old',1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let payload = serde_json::json!({"id":"M","thread_id":"T","action":"first","parts":[],
        "part_id":null,"old":null,"waivers":[],"status":"Approved","outdated":false,
        "created_at":"old"});
    sqlx::query(
        "INSERT INTO design_map_proposal
         (id,project_id,thread_id,operation_id,entry_id,action,payload_json,vision_revision,
          map_revision,status,standards_version,created_at,seq)
         VALUES ('M','P','T','O','E','first',?,0,0,'Approved',1,'old',1)",
    )
    .bind(payload.to_string())
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;
    let storage = Storage::open(&path).await.unwrap();
    let project = ProjectId::from_literal("P");
    let vision = storage.list_vision_proposals(&project).await.unwrap();
    let maps = storage.list_map_proposals(&project).await.unwrap();
    for by in [
        vision[0].approved_by.as_ref().unwrap(),
        maps[0].approved_by.as_ref().unwrap(),
    ] {
        assert_eq!(by.agent, "human");
        assert_eq!(by.principal, "user:local");
        assert!(by.request.is_none());
    }
}

/// Runs only on the externally prepared SQLite backup; never on the live DB.
#[tokio::test]
#[ignore = "requires SHADOWS_DESIGN_MIGRATION_COPY made through SQLite backup"]
async fn migrate_prepared_dev_copy_only() {
    let path = std::path::PathBuf::from(
        std::env::var("SHADOWS_DESIGN_MIGRATION_COPY").expect("prepared backup path"),
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.superpowers/sdd/2026-10-03-planning-workspace-stage-1/task-4-runtime")
        .canonicalize()
        .unwrap();
    let copy = path.canonicalize().unwrap();
    assert!(
        copy.starts_with(root) && copy.file_name().unwrap() == "dev-copy.sqlite3",
        "only this plan's disposable copy is permitted"
    );
    // Storage's URL uses a regular Windows path, not the verbatim \\?\ prefix.
    let storage = Storage::open(dunce::simplified(&copy)).await.unwrap();
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(storage.reader())
            .await
            .unwrap()
            .is_empty()
    );
    storage.reader().close().await;
}

async fn old_rows(pool: &sqlx::SqlitePool) -> Vec<Vec<String>> {
    // Text JSON fields are strings here: even their original whitespace must survive.
    let queries = [
        "SELECT json_array(id,project_id,state,created_at,archived_at) FROM plan ORDER BY id",
        concat!(
            "SELECT json_array(id,plan_id,state,previous_version_id,source_plan_json,",
            "version,revision,title,goal,change_reason,written_by_thread,",
            "written_by_operation,written_by_grant,created_at,updated_at,frozen_at) ",
            "FROM workflow ORDER BY id",
        ),
        concat!(
            "SELECT json_array(id,workflow_id,number,contract_json,scope_json,",
            "created_at,updated_at) FROM task ORDER BY id",
        ),
        concat!(
            "SELECT json_array(seq,event_id,kind,project_id,thread_id,operation_id,",
            "actor_kind,actor_id,causation_kind,causation_ref,correlation_id,",
            "payload_json,created_at) FROM durable_event ",
            "WHERE event_id='old-event' ORDER BY seq",
        ),
    ];
    let mut rows = Vec::new();
    for query in queries {
        rows.push(sqlx::query_scalar(query).fetch_all(pool).await.unwrap());
    }
    rows
}

#[tokio::test]
async fn workspace_migrations_preserve_old_frozen_bytes_and_reopened_links() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("old.sqlite3");
    let opts = SqliteConnectOptions::from_str(&format!(
        "sqlite://{}",
        path.to_string_lossy().replace('\\', "/")
    ))
    .unwrap()
    .create_if_missing(true)
    .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    let mut old = sqlx::migrate!("./migrations");
    old.migrations = Cow::Owned(
        old.migrations
            .iter()
            .filter(|m| m.version <= 12)
            .cloned()
            .collect(),
    );
    old.run(&pool).await.unwrap();
    sqlx::raw_sql(
        r#"INSERT INTO project(id,slug,name,directory,created_at) VALUES
         ('00000000-0000-4000-8000-000000000001','old','قديم','C:/old','2026-09-01');
         INSERT INTO planning_thread(id,project_id,title,status,created_at,title_source) VALUES
         ('T','00000000-0000-4000-8000-000000000001','كاتب','Open','2026-09-01','client');
         INSERT INTO plan(id,project_id,state,created_at,archived_at) VALUES
         ('00000000-0000-4000-8000-000000000002',
          '00000000-0000-4000-8000-000000000001','Archived','2026-09-01','2026-09-02');
         INSERT INTO workflow(id,plan_id,state,version,revision,title,goal,
          written_by_thread,created_at,updated_at,source_plan_json) VALUES
         ('W','00000000-0000-4000-8000-000000000002','Draft',1,2,'دخول',' آمن ',
          'T','2026-09-01','2026-09-01','{ "text": "أصل" }');
         INSERT INTO task(id,workflow_id,number,contract_json,scope_json,
          created_at,updated_at) VALUES
         ('K','W',1,'{ "exact": "نص  " }','{ "write": ["src/auth.rs"] }',
          '2026-09-01','2026-09-01');
         UPDATE workflow SET state='Frozen', frozen_at='2026-09-01' WHERE id='W';
         INSERT INTO durable_event(event_id,kind,project_id,thread_id,actor_kind,actor_id,
          payload_json,created_at) VALUES
         ('old-event','WorkflowFrozen','00000000-0000-4000-8000-000000000001','T','User',
          'local','{ "workflow_id": "W" }','2026-09-01');"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = old_rows(&pool).await;
    assert!(before.iter().all(|rows| rows.len() == 1));
    pool.close().await;

    let storage = Storage::open(&path).await.unwrap();
    assert_eq!(old_rows(storage.reader()).await, before);
    let audit: (String, String) =
        sqlx::query_as("SELECT approved_principal,approved_agent FROM workflow WHERE id='W'")
            .fetch_one(storage.reader())
            .await
            .unwrap();
    assert_eq!(audit, ("user:local".into(), "human".into()));
    assert!(
        sqlx::query("UPDATE workflow SET title='illegal' WHERE id='W'")
            .execute(storage.reader())
            .await
            .is_err(),
        "frozen guard survives backfill"
    );
    let faults = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(storage.reader())
        .await
        .unwrap();
    assert!(faults.is_empty());
    let project = ProjectId::from_literal(PROJECT);
    assert_eq!(storage.design_vision(&project).await.unwrap().revision, 0);
    assert!(
        storage
            .design_parts(&project, None, None)
            .await
            .unwrap()
            .items
            .is_empty()
    );
    let a = PartId::generate();
    storage.seed_map_for_tests(&project).await.unwrap();
    let b = PartId::generate();
    let child = PartId::generate();
    let outcome = OutcomeId::generate();
    let make_part = |id: &PartId, parent: Option<PartId>, title: &str| DesignOp::PartCreate {
        id: id.clone(),
        parent,
        before: None,
        content: PartContent {
            serves: vec![],
            folders: vec![],
            files: vec![],
            contract: Default::default(),
            title: title.into(),
            responsibility: "مسؤولية".into(),
            design: "تصميم\n".into(),
            kind: None,
        },
    };
    let ops = vec![
        make_part(&a, None, "باكند"),
        make_part(&b, None, "واجهة"),
        make_part(&child, Some(a.clone()), "Schema"),
        DesignOp::PartMove {
            id: b.clone(),
            parent: None,
            before: Some(a.clone()),
        },
        DesignOp::OutcomeCreate {
            id: outcome.clone(),
            parent: None,
            before: None,
            content: OutcomeContent {
                title: "دخول".into(),
                intended_result: "تجربة آمنة".into(),
                acceptance: vec!["يدخل".into()],
            },
        },
        DesignOp::OutcomePartPut {
            outcome: outcome.clone(),
            part: a.clone(),
        },
        DesignOp::OutcomePartPut {
            outcome: outcome.clone(),
            part: b.clone(),
        },
        DesignOp::PlanLinkPut {
            anchor: DesignAnchor::Outcome(outcome.clone()),
            plan: PlanId::from_literal(PLAN),
        },
    ];
    let command = CommandContext {
        agent: None,
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: "migration-edit".into(),
        command_kind: "DesignEdit".into(),
        command_schema_ver: 1,
        request_fingerprint: "migration-test".into(),
    };
    let result = storage
        .edit_design(&command, &project, 0, ops.clone())
        .await
        .unwrap();
    let detail = storage.design_outcome(&project, &outcome).await.unwrap();
    assert_eq!(detail.parts.len(), 2);
    assert_eq!(detail.plans, vec![PlanId::from_literal(PLAN)]);
    assert_eq!(old_rows(storage.reader()).await, before);
    storage.reader().close().await;
    drop(storage);

    let reopened = Storage::open(&path).await.unwrap();
    let roots = reopened.design_parts(&project, None, None).await.unwrap();
    assert_eq!(roots.revision, 1);
    assert_eq!(
        roots.items.iter().map(|p| &p.id).collect::<Vec<_>>(),
        vec![&b, &a]
    );
    let child_view = reopened.design_part(&project, &child).await.unwrap();
    assert_eq!(child_view.ancestors[0].id, a);
    assert_eq!(
        reopened.design_outcome(&project, &outcome).await.unwrap(),
        detail
    );
    assert_eq!(
        reopened
            .edit_design(&command, &project, 0, ops)
            .await
            .unwrap(),
        result
    );
    assert_eq!(old_rows(reopened.reader()).await, before);
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(reopened.reader())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        sqlx::query("UPDATE workflow SET title='changed' WHERE id='W'")
            .execute(reopened.reader())
            .await
            .unwrap_err()
            .to_string()
            .contains("frozen plan version never changes")
    );
}

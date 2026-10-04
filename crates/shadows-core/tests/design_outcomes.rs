use shadows_core::testing::{
    CommandContext, LiveHandles, Runtime, Storage, Writer, acp, fingerprint,
};
use shadows_core::{
    AcceptanceItem, AppCore, CoreParts, DesignAnchor, DesignOp, OutcomeContent, OutcomeId,
    PartContent, PartId, PlanOp, ProjectId, TaskContent,
};
use std::sync::Arc;

fn ctx(writer: &Writer, id: &str, kind: &str) -> CommandContext {
    let (principal_kind, principal_id) = writer.principal();
    CommandContext {
        principal_kind: principal_kind.into(),
        principal_id,
        command_id: id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, &serde_json::json!({"id":id})),
    }
}
async fn fixture(path: &std::path::Path) -> (Arc<Storage>, Arc<AppCore>) {
    let storage = Arc::new(Storage::open(path).await.unwrap());
    let runtime = Arc::new(Runtime::start(storage.clone()).await.unwrap().0);
    let core = AppCore::assemble(CoreParts {
        storage: storage.clone(),
        runtime,
        sessions: acp::fake_sessions(storage.clone()),
        handles: Arc::new(LiveHandles::default()),
        bus: tokio::sync::broadcast::channel(32).0,
        ui: tokio::sync::broadcast::channel(32).0,
        mcp_url: acp::MCP_URL.into(),
    });
    (storage, core)
}
async fn project(core: &AppCore, dir: &std::path::Path, slug: &str) -> ProjectId {
    core.projects()
        .create(format!("project-{slug}"), slug, slug, dir.to_str().unwrap())
        .await
        .unwrap()
        .id
}
fn outcome(id: &OutcomeId, parent: Option<&OutcomeId>, title: &str) -> DesignOp {
    DesignOp::OutcomeCreate {
        id: id.clone(),
        parent: parent.cloned(),
        before: None,
        content: OutcomeContent {
            title: title.into(),
            intended_result: " دخول آمن ".into(),
            acceptance: vec!["المستخدم يدخل".into(), "يمكنه الخروج".into()],
        },
    }
}
fn part(id: &PartId, title: &str) -> DesignOp {
    DesignOp::PartCreate {
        id: id.clone(),
        parent: None,
        before: None,
        content: PartContent {
            title: title.into(),
            responsibility: "جزء".into(),
            design: "تصميم".into(),
            kind: None,
        },
    }
}
async fn edit(core: &AppCore, p: &ProjectId, rev: i64, ops: Vec<DesignOp>) {
    assert_eq!(
        core.design()
            .edit(format!("edit-{p}-{rev}"), p, rev, ops)
            .await
            .unwrap()
            .revision,
        rev + 1
    );
}
async fn draft(
    storage: &Storage,
    core: &AppCore,
    p: &ProjectId,
    n: &str,
) -> (shadows_core::DraftStarted, shadows_core::ThreadId) {
    let thread = core
        .threads()
        .create(format!("thread-{n}"), p, n, None)
        .await
        .unwrap()
        .id;
    let grant = storage.issue_thread_grant(&thread).await.unwrap().0;
    let writer = Writer::Planner {
        thread: thread.clone(),
        grant: grant.id,
    };
    let started = storage
        .start_draft(
            &ctx(&writer, &format!("draft-{n}"), "DraftStart"),
            &writer,
            p,
            None,
            None,
            Some((n, "goal")),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    (started, thread)
}

// Missing project-qualified association checks or copied parts fail here.
#[tokio::test]
async fn outcome_spans_parts_without_copying_them() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, core) = fixture(&tmp.path().join("test.sqlite3")).await;
    let p = project(&core, tmp.path(), "one").await;
    let qdir = tmp.path().join("q");
    std::fs::create_dir(&qdir).unwrap();
    let q = project(&core, &qdir, "two").await;
    let (a, b, foreign, o) = (
        PartId::generate(),
        PartId::generate(),
        PartId::generate(),
        OutcomeId::generate(),
    );
    edit(
        &core,
        &p,
        0,
        vec![
            part(&a, "Backend"),
            part(&b, "Frontend"),
            outcome(&o, None, " دخول "),
        ],
    )
    .await;
    edit(&core, &q, 0, vec![part(&foreign, "Other")]).await;
    let (v1, _) = draft(&storage, &core, &p, "backend").await;
    let (v2, _) = draft(&storage, &core, &p, "frontend").await;
    edit(
        &core,
        &p,
        1,
        vec![
            DesignOp::OutcomePartPut {
                outcome: o.clone(),
                part: a.clone(),
            },
            DesignOp::OutcomePartPut {
                outcome: o.clone(),
                part: b.clone(),
            },
            DesignOp::OutcomePartPut {
                outcome: o.clone(),
                part: a.clone(),
            },
            DesignOp::PlanLinkPut {
                anchor: DesignAnchor::Outcome(o.clone()),
                plan: v1.plan_id.clone(),
            },
            DesignOp::PlanLinkPut {
                anchor: DesignAnchor::Outcome(o.clone()),
                plan: v2.plan_id.clone(),
            },
        ],
    )
    .await;
    let view = core.design().outcome(&p, &o).await.unwrap();
    assert_eq!(view.outcome.content.title, "دخول");
    assert_eq!(view.outcome.revision, 2);
    assert_eq!(view.parts.len(), 2);
    assert_eq!(view.plans.len(), 2);
    assert!(view.parts.contains(&a) && view.parts.contains(&b));
    edit(
        &core,
        &p,
        2,
        vec![DesignOp::PartPut {
            id: a.clone(),
            content: PartContent {
                title: "الخدمات".into(),
                responsibility: "جزء".into(),
                design: "تصميم جديد".into(),
                kind: None,
            },
        }],
    )
    .await;
    assert!(
        core.design()
            .outcome(&p, &o)
            .await
            .unwrap()
            .parts
            .contains(&a)
    );
    assert_eq!(
        core.design().part(&p, &a).await.unwrap().part.content.title,
        "الخدمات"
    );
    assert!(
        core.design()
            .edit(
                "foreign".into(),
                &p,
                3,
                vec![
                    DesignOp::OutcomePut {
                        id: o.clone(),
                        content: OutcomeContent {
                            title: "must rollback".into(),
                            intended_result: "".into(),
                            acceptance: vec![]
                        }
                    },
                    DesignOp::OutcomePartPut {
                        outcome: o.clone(),
                        part: foreign
                    }
                ]
            )
            .await
            .is_err()
    );
    assert_eq!(
        core.design()
            .outcome(&p, &o)
            .await
            .unwrap()
            .outcome
            .content
            .title,
        "دخول"
    );
    assert_eq!(core.design().vision(&p).await.unwrap().revision, 3);
    edit(
        &core,
        &p,
        3,
        vec![
            DesignOp::OutcomePartRemove {
                outcome: o.clone(),
                part: a.clone(),
            },
            DesignOp::PlanLinkRemove {
                anchor: DesignAnchor::Outcome(o.clone()),
                plan: v1.plan_id,
            },
        ],
    )
    .await;
    let view = core.design().outcome(&p, &o).await.unwrap();
    assert_eq!(view.parts, vec![b]);
    assert_eq!(view.plans, vec![v2.plan_id]);
}

// Shared parent trees, lost ordering and stale moves fail here.
#[tokio::test]
async fn outcome_hierarchy_is_independent_of_parts() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.sqlite3");
    let (_, core) = fixture(&db).await;
    let p = project(&core, tmp.path(), "one").await;
    let (a, b, c) = (
        OutcomeId::generate(),
        OutcomeId::generate(),
        OutcomeId::generate(),
    );
    let part_id = PartId::generate();
    edit(
        &core,
        &p,
        0,
        vec![
            outcome(&a, None, "A"),
            outcome(&b, Some(&a), "B"),
            outcome(&c, Some(&b), "C"),
            part(&part_id, "Root part"),
        ],
    )
    .await;
    assert!(
        core.design()
            .edit(
                "cycle".into(),
                &p,
                1,
                vec![DesignOp::OutcomeMove {
                    id: a.clone(),
                    parent: Some(c.clone()),
                    before: None
                }]
            )
            .await
            .is_err()
    );
    edit(
        &core,
        &p,
        1,
        vec![DesignOp::OutcomeMove {
            id: c.clone(),
            parent: None,
            before: Some(a.clone()),
        }],
    )
    .await;
    assert!(
        core.design()
            .edit(
                "stale".into(),
                &p,
                1,
                vec![DesignOp::OutcomeMove {
                    id: b.clone(),
                    parent: None,
                    before: None
                }]
            )
            .await
            .is_err()
    );
    assert!(
        core.design()
            .part(&p, &part_id)
            .await
            .unwrap()
            .part
            .parent
            .is_none()
    );
    assert_eq!(
        core.design().outcome(&p, &b).await.unwrap().ancestors[0].id,
        a
    );
    core.code().shut_down_for_test().await;
    drop(core);
    let (_, reopened) = fixture(&db).await;
    assert_eq!(
        reopened
            .design()
            .outcomes(&p, None, None)
            .await
            .unwrap()
            .items
            .iter()
            .map(|o| o.id.clone())
            .collect::<Vec<_>>(),
        vec![c, a]
    );
}

// A plan state must never derive an outcome's completion or mutate Frozen bytes.
#[tokio::test]
async fn plan_lifecycle_never_completes_outcomes() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, core) = fixture(&tmp.path().join("test.sqlite3")).await;
    let p = project(&core, tmp.path(), "one").await;
    let (v, thread) = draft(&storage, &core, &p, "login").await;
    let task = TaskContent {
        number: 1,
        title: "Task".into(),
        goal: "Goal".into(),
        reads: vec![],
        writes: vec!["file.rs".into()],
        acceptance: vec![AcceptanceItem {
            number: 1,
            text: "works".into(),
        }],
    };
    storage
        .edit_plan(
            &ctx(&Writer::Person, "plan-edit", "PlanEdit"),
            &Writer::Person,
            None,
            &v.workflow_id,
            0,
            &[PlanOp::TaskAdd { task }],
        )
        .await
        .unwrap();
    let id = OutcomeId::generate();
    edit(
        &core,
        &p,
        0,
        vec![
            outcome(&id, None, "Login"),
            DesignOp::PlanLinkPut {
                anchor: DesignAnchor::Outcome(id.clone()),
                plan: v.plan_id.clone(),
            },
        ],
    )
    .await;
    let original =
        serde_json::to_value(core.design().outcome(&p, &id).await.unwrap().outcome).unwrap();
    core.plans()
        .approve("approve".into(), &v.workflow_id, 1)
        .await
        .unwrap();
    let frozen = core.plans().get(&v.workflow_id).await.unwrap().content();
    let frozen_record: String = sqlx::query_scalar("SELECT json_object('source',source_plan_json,'title',title,'goal',goal,'state',state,'revision',revision,'frozen_at',frozen_at,'updated_at',updated_at) FROM workflow WHERE id=?")
        .bind(v.workflow_id.as_str()).fetch_one(storage.reader()).await.unwrap();
    let frozen_tasks: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id,contract_json,scope_json FROM task WHERE workflow_id=? ORDER BY id",
    )
    .bind(v.workflow_id.as_str())
    .fetch_all(storage.reader())
    .await
    .unwrap();
    core.plans()
        .archive("archive".into(), &v.plan_id)
        .await
        .unwrap();
    core.threads()
        .remove("delete".into(), &thread)
        .await
        .unwrap();
    let view = core.design().outcome(&p, &id).await.unwrap();
    assert_eq!(view.plans, vec![v.plan_id]);
    assert_eq!(serde_json::to_value(view.outcome).unwrap(), original);
    assert_eq!(
        core.plans().get(&v.workflow_id).await.unwrap().content(),
        frozen
    );
    assert!(original.get("state").is_none());
    assert!(original.get("completed").is_none());
    let after_record: String = sqlx::query_scalar("SELECT json_object('source',source_plan_json,'title',title,'goal',goal,'state',state,'revision',revision,'frozen_at',frozen_at,'updated_at',updated_at) FROM workflow WHERE id=?")
        .bind(v.workflow_id.as_str()).fetch_one(storage.reader()).await.unwrap();
    let after_tasks: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id,contract_json,scope_json FROM task WHERE workflow_id=? ORDER BY id",
    )
    .bind(v.workflow_id.as_str())
    .fetch_all(storage.reader())
    .await
    .unwrap();
    assert_eq!(after_record, frozen_record);
    assert_eq!(after_tasks, frozen_tasks);
}

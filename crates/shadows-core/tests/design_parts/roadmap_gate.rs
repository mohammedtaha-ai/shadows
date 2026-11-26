//! No plan work before an approved roadmap, and no approval of an untied plan.
use super::*;
use shadows_core::{ErrorCode, Grant, OutcomeContent, OutcomeId};

fn blocked<T: std::fmt::Debug>(result: Result<T, CoreError>, reason: &str) {
    match result.unwrap_err() {
        CoreError::Refused {
            code: ErrorCode::StageBlocked,
            message,
        } => assert!(message.contains(reason), "{message}"),
        other => panic!("expected STAGE_BLOCKED {reason}: {other:?}"),
    }
}

async fn setup() -> (tempfile::TempDir, Arc<AppCore>, ProjectId, Grant, Storage) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("gate.sqlite3");
    let core = core_at(&db).await;
    let p = project(&core, "gate", tmp.path()).await;
    let grant = {
        let issued = core
            .grants()
            .issue_external("grant".into(), None, vec![p.clone()])
            .await
            .unwrap();
        core.grants()
            .external_project(&issued.connection.id, &p)
            .await
            .unwrap()
    };
    (tmp, core, p, grant, Storage::open(&db).await.unwrap())
}

async fn start(core: &AppCore, grant: &Grant, parts: Vec<PartId>) -> DraftStart {
    let draft_ref = core
        .plans()
        .prepare_draft(grant)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    DraftStart {
        title: Some("Plan".into()),
        goal: Some("Goal".into()),
        plan_id: None,
        reason: None,
        draft_ref: Some(draft_ref),
        parts,
    }
}

async fn outcomes(core: &AppCore, p: &ProjectId, n: usize) -> Vec<OutcomeId> {
    let ids: Vec<_> = (0..n).map(|_| OutcomeId::generate()).collect();
    let ops = ids
        .iter()
        .map(|id| DesignOp::OutcomeCreate {
            id: id.clone(),
            parent: None,
            before: None,
            content: OutcomeContent {
                title: format!("Outcome {id}"),
                intended_result: "served".into(),
                acceptance: vec![],
            },
        })
        .collect();
    link(core, p, &format!("outcomes-{}", ids[0]), ops)
        .await
        .unwrap();
    ids
}

fn put(outcome: &OutcomeId, plan: &shadows_core::PlanId) -> DesignOp {
    DesignOp::PlanLinkPut {
        anchor: shadows_core::DesignAnchor::Outcome(outcome.clone()),
        plan: plan.clone(),
    }
}

fn remove(outcome: &OutcomeId, plan: &shadows_core::PlanId) -> DesignOp {
    DesignOp::PlanLinkRemove {
        anchor: shadows_core::DesignAnchor::Outcome(outcome.clone()),
        plan: plan.clone(),
    }
}

/// The person's workspace edit at the current revision.
async fn link(
    core: &AppCore,
    p: &ProjectId,
    command: &str,
    ops: Vec<DesignOp>,
) -> Result<shadows_core::DesignChange, CoreError> {
    let revision = core.design().vision(p).await.unwrap().revision;
    core.plans()
        .edit_design(command.into(), p, revision, ops)
        .await
        .map_err(CoreError::from)
}

/// Which of ids serve a plan, and each one's revision.
async fn served(core: &AppCore, p: &ProjectId, ids: &[OutcomeId]) -> (Vec<OutcomeId>, Vec<i64>) {
    let mut served = Vec::new();
    let mut revisions = Vec::new();
    for id in ids {
        let view = core.plans().design_outcome(p, id).await.unwrap();
        revisions.push(view.outcome.revision);
        if !view.plans.is_empty() {
            served.push(id.clone());
        }
    }
    (served, revisions)
}

fn add_task() -> PlanOp {
    PlanOp::TaskAdd {
        task: TaskContent {
            phases: Vec::new(),
            contract: None,
            number: 1,
            title: "Task".into(),
            goal: "Goal".into(),
            reads: vec![],
            writes: vec!["file.rs".into()],
            acceptance: vec![AcceptanceItem {
                number: 1,
                text: "works".into(),
            }],
        },
    }
}

fn edit_args(workflow: &shadows_core::WorkflowId, revision: i64, command: &str) -> PlanEdit {
    PlanEdit {
        workflow_id: Some(workflow.clone()),
        expected_revision: revision,
        command_id: Some(command.into()),
        ops: vec![add_task()],
    }
}

async fn unapprove_roadmap(storage: &Storage, p: &ProjectId) {
    sqlx::query("UPDATE design_workspace SET roadmap_revision=0 WHERE project_id=?")
        .bind(p.as_str())
        .execute(storage.reader())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_plan_cannot_start_before_the_roadmap() {
    let (_tmp, core, p, grant, _storage) = setup().await;
    let args = start(&core, &grant, vec![]).await;
    blocked(
        core.plans()
            .start_draft(&grant, args)
            .await
            .map_err(shadows_core::CoreError::from),
        "no roadmap approved",
    );
    core.design().seed_roadmap_for_tests(&p).await.unwrap();
    let args = start(&core, &grant, vec![]).await;
    core.plans()
        .start_draft(&grant, args)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    _storage.close().await.unwrap();
}

#[tokio::test]
async fn edit_and_approve_wait_for_the_roadmap() {
    let (_tmp, core, p, grant, storage) = setup().await;
    let part = core.design().seed_roadmap_for_tests(&p).await.unwrap();
    let args = start(&core, &grant, vec![part]).await;
    let started = core
        .plans()
        .start_draft(&grant, args)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    unapprove_roadmap(&storage, &p).await;
    blocked(
        core.plans()
            .edit(&grant, edit_args(&started.workflow_id, 0, "edit-1"))
            .await
            .map_err(shadows_core::CoreError::from),
        "no roadmap approved",
    );
    blocked(
        core.plans()
            .approve("approve-0".into(), &started.workflow_id, 0)
            .await
            .map_err(shadows_core::CoreError::from),
        "no roadmap approved",
    );
    let unbind = PlanOp::BindingRemove {
        task: 1,
        agreement_id: shadows_core::AgreementId::generate(),
        role: shadows_core::AgreementRole::Provides,
    };
    blocked(
        core.plans()
            .edit_bindings("bind-0".into(), &started.workflow_id, 0, vec![unbind])
            .await
            .map_err(shadows_core::CoreError::from),
        "no roadmap approved",
    );
    core.design().seed_roadmap_for_tests(&p).await.unwrap();
    core.plans()
        .edit(&grant, edit_args(&started.workflow_id, 0, "edit-2"))
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn a_plan_with_no_part_cannot_be_approved() {
    let (_tmp, core, p, grant, _storage) = setup().await;
    let part = core.design().seed_roadmap_for_tests(&p).await.unwrap();
    let args = start(&core, &grant, vec![]).await;
    let bare = core
        .plans()
        .start_draft(&grant, args)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    core.plans()
        .edit(&grant, edit_args(&bare.workflow_id, 0, "bare-edit"))
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    blocked(
        core.plans()
            .approve("bare-approve".into(), &bare.workflow_id, 1)
            .await
            .map_err(shadows_core::CoreError::from),
        "plan has no part",
    );
    let args = start(&core, &grant, vec![part.clone()]).await;
    let tied = core
        .plans()
        .start_draft(&grant, args)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    core.plans()
        .edit(&grant, edit_args(&tied.workflow_id, 0, "tied-edit"))
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    blocked(
        core.plans()
            .approve("tied-unserved".into(), &tied.workflow_id, 1)
            .await
            .map_err(shadows_core::CoreError::from),
        "plan has no roadmap outcome: link it on the Roadmap page",
    );
    let state = core
        .plans()
        .get_plan_for_test(&tied.workflow_id)
        .await
        .unwrap()
        .state;
    assert_eq!(state, shadows_core::WorkflowState::Draft);
    let outcome = outcomes(&core, &p, 1).await.remove(0);
    link(&core, &p, "serve", vec![put(&outcome, &tied.plan_id)])
        .await
        .unwrap();
    core.plans()
        .approve("tied-approve".into(), &tied.workflow_id, 1)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let view = core
        .plans()
        .design_part(&p, &part)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(view.plans, vec![tied.plan_id]);

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    _storage.close().await.unwrap();
}

#[tokio::test]
async fn a_start_with_an_unknown_part_links_nothing() {
    let (_tmp, core, p, grant, _storage) = setup().await;
    core.design().seed_roadmap_for_tests(&p).await.unwrap();
    let args = start(&core, &grant, vec![PartId::generate()]).await;
    let refused = core
        .plans()
        .start_draft(&grant, args)
        .await
        .map_err(shadows_core::CoreError::from);
    assert!(refused.is_err(), "{refused:?}");
    assert!(
        core.plans()
            .map(&p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .plans
            .is_empty()
    );

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    _storage.close().await.unwrap();
}

#[tokio::test]
async fn hand_outcome_edits_wait_for_the_first_roadmap() {
    let (_tmp, core, p, _grant, _storage) = setup().await;
    let create = || DesignOp::OutcomeCreate {
        id: OutcomeId::generate(),
        parent: None,
        before: None,
        content: OutcomeContent {
            title: "Ship".into(),
            intended_result: "shipped".into(),
            acceptance: vec![],
        },
    };
    blocked(
        core.plans()
            .edit_design("early".into(), &p, 0, vec![create()])
            .await
            .map_err(shadows_core::CoreError::from),
        "the first roadmap comes from the Planner",
    );
    core.design().seed_roadmap_for_tests(&p).await.unwrap();
    core.plans()
        .edit_design("late".into(), &p, 0, vec![create()])
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    _storage.close().await.unwrap();
}

#[tokio::test]
async fn a_recorded_start_still_answers_before_the_roadmap() {
    let (_tmp, core, p, grant, storage) = setup().await;
    core.design().seed_roadmap_for_tests(&p).await.unwrap();
    let args = start(&core, &grant, vec![]).await;
    let draft_ref = args.draft_ref.clone();
    let first = core
        .plans()
        .start_draft(&grant, args)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    unapprove_roadmap(&storage, &p).await;
    let again = DraftStart {
        title: Some("Plan".into()),
        goal: Some("Goal".into()),
        plan_id: None,
        reason: None,
        draft_ref,
        parts: vec![],
    };
    assert_eq!(
        core.plans()
            .start_draft(&grant, again)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap(),
        first
    );

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    storage.close().await.unwrap();
}

#[tokio::test]
async fn an_approved_plan_keeps_its_last_outcome_and_moves_it_whole() {
    let (_tmp, core, p, grant, storage) = setup().await;
    let part = core.design().seed_roadmap_for_tests(&p).await.unwrap();
    let ids = outcomes(&core, &p, 4).await;
    let args = start(&core, &grant, vec![part]).await;
    let plan = core
        .plans()
        .start_draft(&grant, args)
        .await
        .map_err(CoreError::from)
        .unwrap();
    core.plans()
        .edit(&grant, edit_args(&plan.workflow_id, 0, "keep-edit"))
        .await
        .map_err(CoreError::from)
        .unwrap();
    let both = vec![put(&ids[0], &plan.plan_id), put(&ids[1], &plan.plan_id)];
    link(&core, &p, "both", both).await.unwrap();
    core.plans()
        .approve("keep-approve".into(), &plan.workflow_id, 1)
        .await
        .map_err(CoreError::from)
        .unwrap();
    link(&core, &p, "drop-one", vec![remove(&ids[0], &plan.plan_id)])
        .await
        .unwrap();
    let before = served(&core, &p, &ids).await;
    assert_eq!(before.0, vec![ids[1].clone()]);
    let workspace = core.design().vision(&p).await.unwrap().revision;
    blocked(
        link(&core, &p, "drop-last", vec![remove(&ids[1], &plan.plan_id)]).await,
        "keeps at least one roadmap outcome",
    );
    assert_eq!(served(&core, &p, &ids).await, before);
    assert_eq!(core.design().vision(&p).await.unwrap().revision, workspace);
    // Remove then put, and put then remove: each batch is checked whole.
    let moved = vec![remove(&ids[1], &plan.plan_id), put(&ids[2], &plan.plan_id)];
    link(&core, &p, "move-1", moved).await.unwrap();
    let moved = vec![put(&ids[3], &plan.plan_id), remove(&ids[2], &plan.plan_id)];
    link(&core, &p, "move-2", moved).await.unwrap();
    assert_eq!(served(&core, &p, &ids).await.0, vec![ids[3].clone()]);

    // A Draft-only plan stays editable down to no outcome.
    let args = start(&core, &grant, vec![]).await;
    let draft = core
        .plans()
        .start_draft(&grant, args)
        .await
        .map_err(CoreError::from)
        .unwrap();
    link(&core, &p, "draft-put", vec![put(&ids[0], &draft.plan_id)])
        .await
        .unwrap();
    link(
        &core,
        &p,
        "draft-drop",
        vec![remove(&ids[0], &draft.plan_id)],
    )
    .await
    .unwrap();

    core.shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    storage.close().await.unwrap();
}

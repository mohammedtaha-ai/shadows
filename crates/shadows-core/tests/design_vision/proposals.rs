use super::*;
use shadows_core::{Grant, ProposalStatus, SendTurn, VisionAction, VisionField, VisionPropose};

struct Fixture {
    _tmp: tempfile::TempDir,
    core: Arc<AppCore>,
    storage: Storage,
    p: ProjectId,
    grant: Grant,
}

impl Fixture {
    async fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let (core, p) = standards::core_and_project(&tmp).await;
        let thread = core
            .threads()
            .create("thread".into(), &p, "Vision", None, None)
            .await
            .unwrap();
        core.turns()
            .send(
                thread.id.clone(),
                SendTurn {
                    attachments: Vec::new(),
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
        let storage = Storage::open(&tmp.path().join("standards.sqlite3"))
            .await
            .unwrap();
        let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&storage, &thread.id)
            .await
            .unwrap();
        Self {
            _tmp: tmp,
            core,
            storage,
            p,
            grant,
        }
    }
    async fn propose(&self, args: VisionPropose) -> shadows_core::VisionProposal {
        self.core
            .design()
            .propose_vision(&self.grant, args)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
    }
    async fn approve(&self, cmd: &str, id: &str) -> shadows_core::DesignChange {
        let change = self
            .core
            .design()
            .approve_vision_proposal(cmd.into(), &self.p, id)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap();
        let proposals = self
            .core
            .design()
            .vision_proposals(&self.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap();
        let p = proposals.iter().find(|p| p.id == id).unwrap();
        let json = serde_json::to_value(p).unwrap();
        assert_eq!(json["approved_by"]["agent"], "human");
        assert_eq!(json["approved_by"]["principal"], "user:local");
        assert!(json["approved_by"]["request"].is_null());
        let audit: (String, String) = sqlx::query_as(
            "SELECT audit_principal,audit_agent FROM command_record WHERE command_id=?",
        )
        .bind(cmd)
        .fetch_one(self.storage.reader())
        .await
        .unwrap();
        assert_eq!(audit, ("user:local".into(), "human".into()));
        change
    }
    async fn close(self) {
        self.core
            .shut_down(std::time::Duration::from_secs(3), std::future::pending())
            .await
            .unwrap();
    }
}

fn add(text: &str) -> VisionPropose {
    VisionPropose {
        action: VisionAction::Add,
        field: Some(VisionField::Goals),
        item_id: None,
        text: Some(text.into()),
        replaces: None,
    }
}

async fn refused_authority(end_turn: bool) {
    use shadows_core::testing::{CommandContext, Writer};
    let f = Fixture::new().await;
    let p = f.propose(add("Never approve late")).await;
    let thread = f.grant.thread_id.as_ref().unwrap();
    let op: String =
        sqlx::query_scalar("SELECT id FROM operation WHERE thread_id=? AND status_kind='Running'")
            .bind(thread.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    let turn = shadows_core::OperationId::from_literal(&op);
    if end_turn {
        f.core.turns().stop(&turn).await.unwrap();
    } else {
        sqlx::query("UPDATE mcp_grant SET revoked_at='2026-10-12' WHERE id=?")
            .bind(f.grant.id.as_str())
            .execute(f.storage.reader())
            .await
            .unwrap();
    }
    let ctx = CommandContext {
        agent: None,
        principal_kind: "Thread".into(),
        principal_id: thread.to_string(),
        command_id: "forbidden-approval".into(),
        command_kind: "VisionProposalApprove".into(),
        command_schema_ver: 1,
        request_fingerprint: "fixture".into(),
    };
    let approver = shadows_core::Approver::Planner {
        writer: Writer::Planner {
            thread: thread.clone(),
            grant: f.grant.id.clone(),
        },
        turn: turn.to_string(),
        harness: "claude-code".into(),
    };
    assert!(
        f.core
            .design()
            .approve_vision_proposal_for_test(&ctx, &f.p, &p.id, &approver)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    assert_eq!(
        f.core
            .design()
            .vision_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()[0]
            .status,
        ProposalStatus::Pending
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM command_record WHERE command_id='forbidden-approval'",
    )
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(count, 0);
    f.close().await;
}

#[tokio::test]
async fn a_revoked_grant_cannot_approve() {
    refused_authority(false).await;
}

#[tokio::test]
async fn an_ended_turn_cannot_approve() {
    refused_authority(true).await;
}

#[tokio::test]
async fn a_new_proposal_cannot_commit_under_a_finished_turn() {
    use shadows_core::testing::{CommandContext, Writer};
    let f = Fixture::new().await;
    let thread = f.grant.thread_id.as_ref().unwrap();
    let operation: String =
        sqlx::query_scalar("SELECT id FROM operation WHERE thread_id=? AND status_kind='Running'")
            .bind(thread.as_str())
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    let operation = shadows_core::OperationId::from_literal(&operation);
    f.core.turns().stop(&operation).await.unwrap();
    let args = add("Must not be stored");
    let ctx = CommandContext {
        agent: None,
        principal_kind: "Thread".into(),
        principal_id: thread.to_string(),
        command_id: "late".into(),
        command_kind: "VisionPropose".into(),
        command_schema_ver: 1,
        request_fingerprint: CommandContext::fingerprint(
            "VisionPropose",
            &serde_json::to_value(&args).unwrap(),
        ),
    };
    let writer = Writer::Planner {
        thread: thread.clone(),
        grant: f.grant.id.clone(),
    };
    let proposal_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM durable_event WHERE kind='VisionProposed' AND project_id=?",
    )
    .bind(f.p.as_str())
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert!(
        f.core
            .design()
            .propose_vision_for_test(&ctx, &writer, &f.p, &operation, args)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM durable_event WHERE kind='VisionProposed' AND project_id=?",
    )
    .bind(f.p.as_str())
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert_eq!(after, proposal_events);
    assert!(
        f.core
            .design()
            .vision_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .is_empty()
    );
    f.close().await;
}
fn edit(text: &str) -> VisionPropose {
    VisionPropose {
        action: VisionAction::Edit,
        field: None,
        item_id: Some("G1".into()),
        text: Some(text.into()),
        replaces: None,
    }
}
async fn hand(f: &Fixture, revision: i64, text: &str) {
    let op = if revision == 0 {
        DesignOp::VisionItemAdd {
            field: VisionField::Goals,
            text: text.into(),
        }
    } else {
        DesignOp::VisionItemPut {
            id: "G1".into(),
            text: text.into(),
        }
    };
    f.core
        .plans()
        .edit_design(format!("hand-{revision}"), &f.p, revision, vec![op])
        .await
        .unwrap();
}

#[tokio::test]
async fn a_proposal_is_a_pending_card_in_its_conversation() {
    let f = Fixture::new().await;
    let p = f.propose(add("Ship in a week")).await;
    assert_eq!(p.status, ProposalStatus::Pending);
    assert_eq!(p.item_id, None);
    assert!(!p.outdated);
    assert_eq!(
        f.core
            .design()
            .vision(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .revision,
        0
    );
    let (kind, body, card): (String, String, String) =
        sqlx::query_as("SELECT kind,body,card_json FROM thread_entry WHERE kind='VisionProposal'")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(kind, "VisionProposal");
    assert_eq!(body, "Add goal: Ship in a week");
    let json: serde_json::Value = serde_json::from_str(&card).unwrap();
    assert_eq!(json["id"], p.id);
    assert_eq!(json["text"], "Ship in a week");
    let cursor = shadows_core::Events::current_cursor_for_test(&f.storage)
        .await
        .unwrap()
        .0;
    assert_eq!(f.propose(add("Ship in a week")).await, p);
    assert_eq!(
        shadows_core::Events::current_cursor_for_test(&f.storage)
            .await
            .unwrap()
            .0,
        cursor
    );
    f.close().await;
}

#[tokio::test]
async fn two_adds_approve_in_any_order_with_consecutive_ids() {
    let f = Fixture::new().await;
    let a = f.propose(add("A")).await;
    let b = f.propose(add("B")).await;
    f.approve("b", &b.id).await;
    f.approve("a", &a.id).await;
    let v = f
        .core
        .design()
        .vision(&f.p)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(
        v.content
            .goals
            .iter()
            .map(|i| (i.id.as_str(), i.text.as_str()))
            .collect::<Vec<_>>(),
        [("G1", "B"), ("G2", "A")]
    );
    assert!(
        v.content
            .goals
            .iter()
            .all(|i| i.approved_from == f.grant.thread_id)
    );
    assert_eq!(v.revision, 2);
    assert!(
        f.core
            .design()
            .vision_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .iter()
            .all(|p| p.status == ProposalStatus::Approved)
    );
    f.close().await;
}

#[tokio::test]
async fn an_edit_proposal_is_outdated_after_a_hand_edit_of_its_item() {
    let f = Fixture::new().await;
    hand(&f, 0, "Old").await;
    let p = f.propose(edit("New")).await;
    hand(&f, 1, "Mine").await;
    assert!(
        f.core
            .design()
            .vision_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()[0]
            .outdated
    );
    assert!(matches!(
            f.core.design().approve_vision_proposal("bad".into(), &f.p, &p.id)
                .await.map_err(shadows_core::CoreError::from),
        Err(CoreError::Refused { code: shadows_core::ErrorCode::InvalidCommand, message })
            if message.contains("G1 changed after this proposal")));
    assert_eq!(
        f.core
            .design()
            .vision(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .content
            .goals[0]
            .text,
        "Mine"
    );
    f.close().await;
}

#[tokio::test]
async fn an_item_edited_back_to_its_old_text_still_outdates_the_proposal() {
    let f = Fixture::new().await;
    hand(&f, 0, "A").await;
    let p = f.propose(edit("C")).await;
    hand(&f, 1, "B").await;
    hand(&f, 2, "A").await;
    assert!(
        f.core
            .design()
            .vision_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()[0]
            .outdated
    );
    assert!(
        f.core
            .design()
            .approve_vision_proposal("bad".into(), &f.p, &p.id)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    f.close().await;
}

#[tokio::test]
async fn approval_is_scoped_to_its_live_project() {
    let f = Fixture::new().await;
    let p = f.propose(add("A")).await;
    let q = f
        .core
        .project_lifecycle()
        .create("q".into(), "q", "Q", f._tmp.path().to_str().unwrap())
        .await
        .unwrap()
        .id;
    assert!(matches!(
        f.core
            .design()
            .approve_vision_proposal("wrong".into(), &q, &p.id)
            .await
            .map_err(shadows_core::CoreError::from),
        Err(CoreError::Storage(StorageError::NotFound(_)))
    ));
    f.core
        .turns()
        .remove_thread("remove-thread".into(), f.grant.thread_id.as_ref().unwrap())
        .await
        .unwrap();
    f.core
        .project_lifecycle()
        .remove("remove".into(), &f.p)
        .await
        .unwrap();
    let cursor = shadows_core::Events::current_cursor_for_test(&f.storage)
        .await
        .unwrap()
        .0;
    assert!(matches!(
        f.core
            .design()
            .approve_vision_proposal("removed".into(), &f.p, &p.id)
            .await
            .map_err(shadows_core::CoreError::from),
        Err(CoreError::Storage(StorageError::NotFound(_)))
    ));
    assert_eq!(
        shadows_core::Events::current_cursor_for_test(&f.storage)
            .await
            .unwrap()
            .0,
        cursor
    );
    f.close().await;
}

#[tokio::test]
async fn two_concurrent_approvals_write_once() {
    let f = Fixture::new().await;
    let p = f.propose(add("A")).await;
    let (a, b) = tokio::join!(
        f.core
            .design()
            .approve_vision_proposal("a".into(), &f.p, &p.id),
        f.core
            .design()
            .approve_vision_proposal("b".into(), &f.p, &p.id)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert_eq!(
        f.core
            .design()
            .vision(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .revision,
        1
    );
    f.close().await;
}

#[tokio::test]
async fn approving_one_card_does_not_outdate_its_neighbours() {
    let f = Fixture::new().await;
    hand(&f, 0, "x").await;
    let a = f.propose(add("y")).await;
    let b = f.propose(edit("z")).await;
    f.approve("a", &a.id).await;
    assert!(
        !f.core
            .design()
            .vision_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()[1]
            .outdated
    );
    f.approve("b", &b.id).await;
    f.close().await;
}

#[tokio::test]
async fn a_newer_proposal_supersedes_by_item_or_by_replaces() {
    let f = Fixture::new().await;
    hand(&f, 0, "x").await;
    let p = f.propose(edit("a")).await;
    f.propose(edit("b")).await;
    let a = f.propose(add("u")).await;
    f.propose(VisionPropose {
        replaces: Some(a.id.clone()),
        ..add("v")
    })
    .await;
    let ps = f
        .core
        .design()
        .vision_proposals(&f.p)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(ps[0].status, ProposalStatus::Superseded);
    assert_eq!(ps[2].status, ProposalStatus::Superseded);
    assert!(
        f.core
            .design()
            .approve_vision_proposal("old".into(), &f.p, &p.id)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    f.close().await;
}

#[tokio::test]
async fn approving_twice_with_one_command_writes_once() {
    let f = Fixture::new().await;
    let p = f.propose(add("A")).await;
    let first = f.approve("ok", &p.id).await;
    let cursor = shadows_core::Events::current_cursor_for_test(&f.storage)
        .await
        .unwrap()
        .0;
    assert_eq!(f.approve("ok", &p.id).await, first);
    assert_eq!(
        shadows_core::Events::current_cursor_for_test(&f.storage)
            .await
            .unwrap()
            .0,
        cursor
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM design_vision_version")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
    f.close().await;
}

#[tokio::test]
async fn a_proposal_names_a_known_item_and_carries_text() {
    let f = Fixture::new().await;
    for args in [
        edit("x"),
        VisionPropose {
            text: None,
            ..add("x")
        },
        VisionPropose {
            action: VisionAction::Remove,
            ..edit("x")
        },
    ] {
        assert!(
            f.core
                .design()
                .propose_vision(&f.grant, args)
                .await
                .map_err(shadows_core::CoreError::from)
                .is_err()
        );
    }
    let issued = f
        .core
        .grants()
        .issue("external".into(), &f.p)
        .await
        .unwrap();
    assert!(matches!(
        f.core
            .design()
            .propose_vision(&issued.grant, add("x"))
            .await
            .map_err(shadows_core::CoreError::from),
        Err(CoreError::Refused {
            code: shadows_core::ErrorCode::GrantScope,
            ..
        })
    ));
    let t = f
        .core
        .threads()
        .create("idle".into(), &f.p, "Idle", None, None)
        .await
        .unwrap();
    let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&f.storage, &t.id)
        .await
        .unwrap();
    assert!(
        f.core
            .design()
            .propose_vision(&grant, add("x"))
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    f.close().await;
}

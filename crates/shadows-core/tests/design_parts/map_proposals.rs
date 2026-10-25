use super::super::*;
use shadows_core::{Grant, MapAction, MapPropose, ProposedPart, ProposedWaiver, SendTurn};

struct Fixture {
    _tmp: tempfile::TempDir,
    storage: Storage,
    core: Arc<AppCore>,
    p: ProjectId,
    grant: Grant,
}
impl Fixture {
    async fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("proposals.sqlite3");
        let core = core_at(&path).await;
        let p = project(&core, "proposals", tmp.path()).await;
        core.plans()
            .edit_design(
                "vision".into(),
                &p,
                0,
                super::super::stage::vision("Readers"),
            )
            .await
            .unwrap();
        let thread = core
            .threads()
            .create("thread".into(), &p, "Map", None, None)
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
        let storage = Storage::open(&path).await.unwrap();
        let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&storage, &thread.id)
            .await
            .unwrap();
        Self {
            _tmp: tmp,
            storage,
            core,
            p,
            grant,
        }
    }
    async fn close(self) {
        self.core
            .shut_down(std::time::Duration::from_secs(3), std::future::pending())
            .await
            .unwrap();
    }
}
fn first() -> MapPropose {
    MapPropose {
        action: MapAction::First,
        part_id: None,
        replaces: None,
        waivers: vec![ProposedWaiver {
            part: "frontend".into(),
            reason: "Terminal tool".into(),
        }],
        parts: ["backend", "database", "api"]
            .into_iter()
            .map(|kind| {
                let mut content = content(kind);
                content.kind = Some(kind.into());
                content.serves = vec!["G1".into()];
                content.folders = vec![kind.into()];
                ProposedPart {
                    key: kind.into(),
                    parent_key: None,
                    parent: None,
                    content,
                }
            })
            .collect(),
    }
}

#[tokio::test]
async fn equivalent_map_retries_replay_the_original_card() {
    let f = Fixture::new().await;
    let original = f
        .core
        .design()
        .propose_map(&f.grant, first())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let mut retry = first();
    retry.parts[0].content.title = " backend ".into();
    retry.parts[0].content.folders = vec!["./backend/".into()];
    let repeated = f
        .core
        .design()
        .propose_map(&f.grant, retry)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(original.id, repeated.id);
    let proposals = f
        .core
        .design()
        .map_proposals(&f.p)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].status, shadows_core::ProposalStatus::Pending);
    f.close().await;
}

#[tokio::test]
async fn first_map_approval_records_waiver_and_replays_once() {
    let f = Fixture::new().await;
    let p = f
        .core
        .design()
        .propose_map(&f.grant, first())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(
        f.core
            .design()
            .parts(&f.p, None, None)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .items
            .is_empty()
    );
    let a = f
        .core
        .design()
        .approve_map_proposal("approve".into(), &f.p, &p.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let b = f
        .core
        .design()
        .approve_map_proposal("approve".into(), &f.p, &p.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(a, b);
    let proposals = f
        .core
        .design()
        .map_proposals(&f.p)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let approved = proposals.iter().find(|x| x.id == p.id).unwrap();
    assert_eq!(
        serde_json::to_value(approved).unwrap()["approved_by"]["agent"],
        "human"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM design_map_version")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 1);
    let reason: (String, String) =
        sqlx::query_as("SELECT reason,approved_by FROM design_part_waiver")
            .fetch_one(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(reason, ("Terminal tool".into(), "local".into()));
    f.close().await;
}

#[tokio::test]
async fn first_map_refuses_an_unserved_goal_without_writes() {
    let f = Fixture::new().await;
    let mut args = first();
    for part in &mut args.parts {
        part.content.serves.clear();
    }
    let p = f
        .core
        .design()
        .propose_map(&f.grant, args)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(matches!(
        f.core
            .design()
            .approve_map_proposal("bad".into(), &f.p, &p.id)
            .await
            .map_err(shadows_core::CoreError::from),
        Err(CoreError::Refused {
            code: shadows_core::ErrorCode::StageBlocked,
            ..
        })
    ));
    assert!(
        f.core
            .design()
            .parts(&f.p, None, None)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .items
            .is_empty()
    );
    f.close().await;
}

#[tokio::test]
async fn first_map_needs_mandatory_kinds_and_waivers_cannot_hide_database() {
    let f = Fixture::new().await;
    let mut args = first();
    args.parts.retain(|p| p.key != "database");
    let p = f
        .core
        .design()
        .propose_map(&f.grant, args.clone())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(matches!(
        f.core
            .design()
            .approve_map_proposal("missing".into(), &f.p, &p.id)
            .await
            .map_err(shadows_core::CoreError::from),
        Err(CoreError::Refused {
            code: shadows_core::ErrorCode::StageBlocked,
            ..
        })
    ));
    args.waivers.push(ProposedWaiver {
        part: "database".into(),
        reason: "skip".into(),
    });
    assert!(
        f.core
            .design()
            .propose_map(&f.grant, args)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    f.close().await;
}

async fn approved(f: &Fixture) -> shadows_core::PartId {
    let p = f
        .core
        .design()
        .propose_map(&f.grant, first())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    f.core
        .design()
        .approve_map_proposal("first".into(), &f.p, &p.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    f.core
        .design()
        .parts(&f.p, None, None)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap()
        .items[0]
        .id
        .clone()
}

fn addition(name: &str) -> MapPropose {
    let mut content = content(name);
    content.serves = vec!["G1".into()];
    MapPropose {
        action: MapAction::Add,
        parts: vec![ProposedPart {
            key: name.into(),
            parent_key: None,
            parent: None,
            content,
        }],
        part_id: None,
        waivers: vec![],
        replaces: None,
    }
}

#[tokio::test]
async fn a_part_change_is_outdated_by_its_served_item_only() {
    let f = Fixture::new().await;
    let id = approved(&f).await;
    let mut args = addition("changed");
    args.action = MapAction::Change;
    args.part_id = Some(id);
    let p = f
        .core
        .design()
        .propose_map(&f.grant, args)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let rev = f
        .core
        .design()
        .vision(&f.p)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap()
        .revision;
    f.core
        .plans()
        .edit_design(
            "G2".into(),
            &f.p,
            rev,
            vec![DesignOp::VisionItemAdd {
                field: shadows_core::VisionField::Goals,
                text: "new goal".into(),
            }],
        )
        .await
        .unwrap();
    assert!(
        !f.core
            .design()
            .map_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .iter()
            .find(|i| i.id == p.id)
            .unwrap()
            .outdated
    );
    f.core
        .plans()
        .edit_design(
            "G1".into(),
            &f.p,
            rev + 1,
            vec![DesignOp::VisionItemPut {
                id: "G1".into(),
                text: "changed goal".into(),
            }],
        )
        .await
        .unwrap();
    assert!(
        f.core
            .design()
            .map_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .iter()
            .find(|i| i.id == p.id)
            .unwrap()
            .outdated
    );
    assert!(
        f.core
            .design()
            .approve_map_proposal("stale".into(), &f.p, &p.id)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    f.close().await;
}

#[tokio::test]
async fn approval_rechecks_vision_completeness_for_an_independent_addition() {
    let f = Fixture::new().await;
    approved(&f).await;
    let p = f
        .core
        .design()
        .propose_map(&f.grant, addition("notifications"))
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    f.core
        .plans()
        .edit_design(
            "empty-users".into(),
            &f.p,
            2,
            vec![DesignOp::VisionItemRemove { id: "U1".into() }],
        )
        .await
        .unwrap();
    assert!(
        !f.core
            .design()
            .map_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .iter()
            .find(|i| i.id == p.id)
            .unwrap()
            .outdated
    );
    assert!(matches!(
        f.core
            .design()
            .approve_map_proposal("incomplete".into(), &f.p, &p.id)
            .await
            .map_err(shadows_core::CoreError::from),
        Err(CoreError::Refused {
            code: shadows_core::ErrorCode::StageBlocked,
            ..
        })
    ));
    assert_eq!(
        f.core
            .design()
            .parts(&f.p, None, None)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .items
            .len(),
        3
    );
    f.close().await;
}

#[tokio::test]
async fn a_second_add_with_the_same_identity_is_refused_at_approval() {
    let f = Fixture::new().await;
    approved(&f).await;
    let a = f
        .core
        .design()
        .propose_map(&f.grant, addition("notifications"))
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let mut second = addition("NOTIFICATIONS");
    second.parts[0].content.design = "other".into();
    let b = f
        .core
        .design()
        .propose_map(&f.grant, second)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    f.core
        .design()
        .approve_map_proposal("add-first".into(), &f.p, &a.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(
        f.core
            .design()
            .approve_map_proposal("add-second".into(), &f.p, &b.id)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    f.close().await;
}

#[tokio::test]
async fn proposals_pin_the_turns_standards_not_newer_additions() {
    let f = Fixture::new().await;
    f.core
        .standards()
        .save_standards_additions(
            "new-standards".into(),
            &f.p,
            shadows_core::StandardsAdditions::default(),
        )
        .await
        .unwrap();
    let p = f
        .core
        .design()
        .propose_map(&f.grant, first())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let pin: Option<String> = sqlx::query_scalar(
        "SELECT standards_additions_version_id FROM design_map_proposal WHERE id=?",
    )
    .bind(&p.id)
    .fetch_one(f.storage.reader())
    .await
    .unwrap();
    assert!(pin.is_none());
    assert!(
        f.core
            .design()
            .map_proposals(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()[0]
            .outdated
    );
    assert!(
        f.core
            .design()
            .approve_map_proposal("stale-standards".into(), &f.p, &p.id)
            .await
            .map_err(shadows_core::CoreError::from)
            .is_err()
    );
    f.close().await;
}

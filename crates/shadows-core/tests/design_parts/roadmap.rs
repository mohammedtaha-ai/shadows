//! §23.5 roadmap proposals: structure first, one card, approval opens the plans stage.
use super::*;
use shadows_core::{
    ErrorCode, Grant, OutcomeContent, OutcomeId, ProposalStatus, ProposedOutcome, RoadmapAction,
    RoadmapPropose, SendTurn, Stage,
};

pub(super) struct Fixture {
    pub(super) _tmp: tempfile::TempDir,
    pub(super) _storage: Storage,
    pub(super) core: Arc<AppCore>,
    pub(super) p: ProjectId,
    pub(super) grant: Grant,
    pub(super) parts: Vec<PartId>,
}

impl Fixture {
    /// A project whose structure is written, with a Planner turn running.
    pub(super) async fn new(write_structure: bool) -> Self {
        let (tmp, storage, core, p, _dir) = super::structure::setup().await;
        if write_structure {
            core.design()
                .create_structure("write".into(), &p)
                .await
                .map_err(shadows_core::CoreError::from)
                .unwrap();
        }
        let thread = core
            .threads()
            .create("thread".into(), &p, "Roadmap", None, None)
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
        let (grant, _) = shadows_core::Grants::issue_thread_grant_for_test(&storage, &thread.id)
            .await
            .unwrap();
        let parts = core
            .design()
            .parts(&p, None, None)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .items
            .into_iter()
            .map(|part| part.id)
            .collect();
        Self {
            _tmp: tmp,
            _storage: storage,
            core,
            p,
            grant,
            parts,
        }
    }

    pub(super) async fn close(self) {
        self.core
            .shut_down(std::time::Duration::from_secs(3), std::future::pending())
            .await
            .unwrap();
    }

    pub(super) fn outcome(
        &self,
        key: &str,
        parent_key: Option<&str>,
        parts: &[usize],
    ) -> ProposedOutcome {
        ProposedOutcome {
            key: key.into(),
            parent_key: parent_key.map(Into::into),
            parent: None,
            content: OutcomeContent {
                title: format!(" {key} "),
                intended_result: format!("{key} works"),
                acceptance: vec![format!("{key} is accepted")],
            },
            parts: parts.iter().map(|n| self.parts[*n].clone()).collect(),
        }
    }

    pub(super) fn first(&self) -> RoadmapPropose {
        RoadmapPropose {
            action: RoadmapAction::First,
            outcomes: vec![
                self.outcome("login", None, &[0, 1]),
                self.outcome("sso", Some("login"), &[1]),
            ],
            outcome_id: None,
            replaces: None,
        }
    }

    pub(super) async fn proposals(&self) -> Vec<shadows_core::RoadmapProposal> {
        self.core
            .design()
            .roadmap_proposals(&self.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
    }

    pub(super) async fn approve_first(&self) -> shadows_core::RoadmapProposal {
        let card = self
            .core
            .design()
            .propose_roadmap(&self.grant, self.first())
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap();
        self.core
            .plans()
            .approve_roadmap_proposal(format!("approve-{}", card.id), &self.p, &card.id)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap();
        card
    }
}

pub(super) fn refused<T: std::fmt::Debug>(
    result: Result<T, CoreError>,
    code: ErrorCode,
    text: &str,
) {
    match result.unwrap_err() {
        CoreError::Refused { code: c, message } => {
            assert_eq!(c, code, "{message}");
            assert!(message.contains(text), "{message}");
        }
        other => panic!("expected {code:?} {text}: {other:?}"),
    }
}

#[tokio::test]
async fn a_roadmap_waits_for_the_structure() {
    let f = Fixture::new(false).await;
    refused(
        f.core
            .design()
            .propose_roadmap(&f.grant, f.first())
            .await
            .map_err(shadows_core::CoreError::from),
        ErrorCode::StageBlocked,
        "press Create structure first",
    );
    assert!(f.proposals().await.is_empty());
    f.close().await;
}

#[tokio::test]
async fn the_button_approves_a_first_roadmap_and_opens_the_plans_stage() {
    let f = Fixture::new(true).await;
    assert_eq!(
        f.core
            .design()
            .stage(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .stage,
        Stage::Roadmap
    );
    let card = f
        .core
        .design()
        .propose_roadmap(&f.grant, f.first())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(card.outcomes[0].content.title, "login");
    assert!(
        f.core
            .design()
            .outcomes(&f.p, None, None)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .items
            .is_empty()
    );
    let change = f
        .core
        .plans()
        .approve_roadmap_proposal("approve".into(), &f.p, &card.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let replay = f
        .core
        .plans()
        .approve_roadmap_proposal("approve".into(), &f.p, &card.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(change, replay);
    assert_eq!(
        f.core
            .design()
            .stage(&f.p)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .stage,
        Stage::Plans
    );
    let roots = f
        .core
        .design()
        .outcomes(&f.p, None, None)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap()
        .items;
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].content.title, "login");
    let view = f
        .core
        .plans()
        .design_outcome(&f.p, &roots[0].id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(view.parts.len(), 2);
    let children = f
        .core
        .design()
        .outcomes(&f.p, Some(&roots[0].id), None)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(children.items[0].content.title, "sso");
    let approved = &f.proposals().await[0];
    assert_eq!(approved.status, ProposalStatus::Approved);
    assert_eq!(approved.approved_by.as_ref().unwrap().agent, "human");
    // The same arguments in the same turn replay the recorded card; a new first is refused.
    let mut again = f.first();
    again.outcomes.truncate(1);
    refused(
        f.core
            .design()
            .propose_roadmap(&f.grant, again)
            .await
            .map_err(shadows_core::CoreError::from),
        ErrorCode::InvalidCommand,
        "a roadmap is approved",
    );
    f.close().await;
}

#[tokio::test]
async fn the_planner_approves_at_the_persons_request() {
    let f = Fixture::new(true).await;
    let card = f
        .core
        .design()
        .propose_roadmap(&f.grant, f.first())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    f.core
        .plans()
        .approve_roadmap_proposal_as_planner(&f.grant, &card.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let by = f.proposals().await[0].approved_by.clone().unwrap();
    assert!(by.agent.starts_with("planner:"), "{}", by.agent);
    assert!(by.request.is_some());
    f.close().await;
}

#[tokio::test]
async fn a_newer_first_or_a_replaces_supersedes_the_pending_card() {
    let f = Fixture::new(true).await;
    let a = f
        .core
        .design()
        .propose_roadmap(&f.grant, f.first())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let mut again = f.first();
    again.outcomes.truncate(1);
    again.outcomes[0].content.intended_result = "other".into();
    let b = f
        .core
        .design()
        .propose_roadmap(&f.grant, again)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let cards = f.proposals().await;
    assert_eq!(cards[0].id, a.id);
    assert_eq!(cards[0].status, ProposalStatus::Superseded);
    assert_eq!(cards[1].status, ProposalStatus::Pending);
    let mut replacing = f.first();
    replacing.outcomes.truncate(1);
    replacing.outcomes[0].content.intended_result = "third".into();
    replacing.replaces = Some(b.id.clone());
    f.core
        .design()
        .propose_roadmap(&f.grant, replacing)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(f.proposals().await[1].status, ProposalStatus::Superseded);
    refused(
        f.core
            .plans()
            .approve_roadmap_proposal("old".into(), &f.p, &a.id)
            .await
            .map_err(shadows_core::CoreError::from),
        ErrorCode::InvalidCommand,
        "superseded",
    );
    f.close().await;
}

#[tokio::test]
async fn a_part_edit_outdates_the_pending_roadmap_and_a_vision_edit_does_not() {
    let f = Fixture::new(true).await;
    let card = f
        .core
        .design()
        .propose_roadmap(&f.grant, f.first())
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let revision = f
        .core
        .design()
        .vision(&f.p)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap()
        .revision;
    let vision = DesignOp::VisionItemAdd {
        field: shadows_core::VisionField::Goals,
        text: "Another goal".into(),
    };
    f.core
        .plans()
        .edit_design("vision".into(), &f.p, revision, vec![vision])
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(!f.proposals().await[0].outdated);
    let part = f
        .core
        .plans()
        .design_part(&f.p, &f.parts[0])
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap()
        .part;
    let mut content = part.content.clone();
    content.design = "edited by hand".into();
    let put = DesignOp::PartPut {
        id: part.id.clone(),
        content,
    };
    let revision = f
        .core
        .design()
        .vision(&f.p)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap()
        .revision;
    f.core
        .plans()
        .edit_design("part".into(), &f.p, revision, vec![put])
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(f.proposals().await[0].outdated);
    refused(
        f.core
            .plans()
            .approve_roadmap_proposal("stale".into(), &f.p, &card.id)
            .await
            .map_err(shadows_core::CoreError::from),
        ErrorCode::StageBlocked,
        "proposal outdated; propose again",
    );
    assert!(
        f.core
            .design()
            .outcomes(&f.p, None, None)
            .await
            .map_err(shadows_core::CoreError::from)
            .unwrap()
            .items
            .is_empty()
    );
    f.close().await;
}

#[tokio::test]
async fn a_malformed_roadmap_is_refused_whole() {
    let f = Fixture::new(true).await;
    let mut cases: Vec<(RoadmapPropose, &str)> = Vec::new();
    let mut no_part = f.first();
    no_part.outcomes[0].parts.clear();
    cases.push((no_part, "at least one part"));
    let mut unknown = f.first();
    unknown.outcomes[0].parts = vec![PartId::generate()];
    cases.push((unknown, "does not belong"));
    let mut blank = f.first();
    blank.outcomes[0].content.title = "  ".into();
    cases.push((blank, "needs a title"));
    let mut duplicate = f.first();
    duplicate.outcomes[1].key = "login".into();
    cases.push((duplicate, "unique slugs"));
    let mut cycle = f.first();
    cycle.outcomes[0].parent_key = Some("sso".into());
    cases.push((cycle, "cycle"));
    let mut missing = f.first();
    missing.action = RoadmapAction::Change;
    missing.outcome_id = Some(OutcomeId::generate());
    missing.outcomes.truncate(1);
    cases.push((missing, "no roadmap yet"));
    for (args, text) in cases {
        let result = f
            .core
            .design()
            .propose_roadmap(&f.grant, args)
            .await
            .map_err(shadows_core::CoreError::from);
        match result.unwrap_err() {
            CoreError::Refused { message, .. } => assert!(message.contains(text), "{message}"),

            other => panic!("{text}: {other:?}"),
        }
    }
    assert!(f.proposals().await.is_empty());
    f.close().await;
}

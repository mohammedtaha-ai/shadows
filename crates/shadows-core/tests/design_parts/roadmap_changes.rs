//! §23.5 roadmap changes after the first: add, change and remove one outcome,
//! each outdated only by its own outcome or the parts it references.
use super::roadmap::{Fixture, refused};
use super::*;
use shadows_core::{ErrorCode, OutcomeId, RoadmapAction, RoadmapPropose};

#[tokio::test]
async fn add_change_and_remove_work_after_the_first_roadmap() {
    let f = Fixture::new(true).await;
    f.approve_first().await;
    let roots = f
        .core
        .design()
        .outcomes(&f.p, None, None)
        .await
        .unwrap()
        .items;
    let login = roots[0].clone();
    let mut add = f.first();
    add.action = RoadmapAction::Add;
    add.outcomes = vec![f.outcome("billing", None, &[2])];
    let added = f
        .core
        .design()
        .propose_roadmap(&f.grant, add)
        .await
        .unwrap();
    f.core
        .plans()
        .approve_roadmap_proposal("add".into(), &f.p, &added.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(
        f.core
            .design()
            .outcomes(&f.p, None, None)
            .await
            .unwrap()
            .items
            .len(),
        2
    );
    let mut change = f.first();
    change.action = RoadmapAction::Change;
    change.outcome_id = Some(login.id.clone());
    change.outcomes = vec![f.outcome("login", None, &[3])];
    let changed = f
        .core
        .design()
        .propose_roadmap(&f.grant, change)
        .await
        .unwrap();
    assert_eq!(changed.old.as_ref().unwrap().title, "login");
    f.core
        .plans()
        .approve_roadmap_proposal("change".into(), &f.p, &changed.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let view = f
        .core
        .plans()
        .design_outcome(&f.p, &login.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(view.parts, vec![f.parts[3].clone()]);
    assert_eq!(view.outcome.revision, login.revision + 1);
    let remove = |outcome: &OutcomeId| RoadmapPropose {
        action: RoadmapAction::Remove,
        outcomes: vec![],
        outcome_id: Some(outcome.clone()),
        replaces: None,
    };
    let blocked = f
        .core
        .design()
        .propose_roadmap(&f.grant, remove(&login.id))
        .await
        .unwrap();
    refused(
        f.core
            .plans()
            .approve_roadmap_proposal("blocked".into(), &f.p, &blocked.id)
            .await
            .map_err(shadows_core::CoreError::from),
        ErrorCode::StageBlocked,
        "outcome has children or plans",
    );
    let billing = f
        .core
        .design()
        .outcomes(&f.p, None, None)
        .await
        .unwrap()
        .items[1]
        .clone();
    let gone = f
        .core
        .design()
        .propose_roadmap(&f.grant, remove(&billing.id))
        .await
        .unwrap();
    f.core
        .plans()
        .approve_roadmap_proposal("gone".into(), &f.p, &gone.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(
        f.core
            .design()
            .outcomes(&f.p, None, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    f.close().await;
}

#[tokio::test]
async fn a_hand_edit_of_the_outcome_outdates_its_pending_change() {
    let f = Fixture::new(true).await;
    f.approve_first().await;
    let login = f
        .core
        .design()
        .outcomes(&f.p, None, None)
        .await
        .unwrap()
        .items[0]
        .clone();
    let mut change = f.first();
    change.action = RoadmapAction::Change;
    change.outcome_id = Some(login.id.clone());
    change.outcomes = vec![f.outcome("login", None, &[0])];
    let card = f
        .core
        .design()
        .propose_roadmap(&f.grant, change)
        .await
        .unwrap();
    let mut content = login.content.clone();
    content.intended_result = "by hand".into();
    let put = DesignOp::OutcomePut {
        id: login.id.clone(),
        content,
    };
    let revision = f.core.design().vision(&f.p).await.unwrap().revision;
    f.core
        .plans()
        .edit_design("hand".into(), &f.p, revision, vec![put])
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(
        f.proposals()
            .await
            .iter()
            .find(|c| c.id == card.id)
            .unwrap()
            .outdated
    );
    f.close().await;
}

#[tokio::test]
async fn a_change_moves_its_outcome_to_the_parent_it_names() {
    let f = Fixture::new(true).await;
    f.approve_first().await;
    let login = f
        .core
        .design()
        .outcomes(&f.p, None, None)
        .await
        .unwrap()
        .items[0]
        .clone();
    let sso = f
        .core
        .design()
        .outcomes(&f.p, Some(&login.id), None)
        .await
        .unwrap()
        .items[0]
        .clone();
    let mut change = f.first();
    change.action = RoadmapAction::Change;
    change.outcome_id = Some(sso.id.clone());
    change.outcomes = vec![f.outcome("sso", None, &[1])];
    let card = f
        .core
        .design()
        .propose_roadmap(&f.grant, change)
        .await
        .unwrap();
    assert_eq!(card.old_parent.as_ref(), Some(&login.id));
    f.core
        .plans()
        .approve_roadmap_proposal("move".into(), &f.p, &card.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let moved = f
        .core
        .plans()
        .design_outcome(&f.p, &sso.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(moved.outcome.parent, None);
    let mut back = f.first();
    back.action = RoadmapAction::Change;
    back.outcome_id = Some(sso.id.clone());
    back.outcomes = vec![f.outcome("sso", None, &[1])];
    back.outcomes[0].parent = Some(login.id.clone());
    let card = f
        .core
        .design()
        .propose_roadmap(&f.grant, back)
        .await
        .unwrap();
    f.core
        .plans()
        .approve_roadmap_proposal("back".into(), &f.p, &card.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let moved = f
        .core
        .plans()
        .design_outcome(&f.p, &sso.id)
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert_eq!(moved.outcome.parent, Some(login.id.clone()));
    f.close().await;
}

#[tokio::test]
async fn tying_a_plan_to_a_part_leaves_a_pending_card_valid() {
    let f = Fixture::new(true).await;
    f.approve_first().await;
    let mut add = f.first();
    add.action = RoadmapAction::Add;
    add.outcomes = vec![f.outcome("billing", None, &[0])];
    let card = f
        .core
        .design()
        .propose_roadmap(&f.grant, add)
        .await
        .unwrap();
    let plan = f
        .core
        .plans()
        .start_draft(
            &f.grant,
            DraftStart {
                title: Some("Plan".into()),
                goal: Some("Goal".into()),
                plan_id: None,
                reason: None,
                draft_ref: None,
                parts: vec![],
            },
        )
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap()
        .plan_id;
    let link = DesignOp::PlanLinkPut {
        anchor: DesignAnchor::Part(f.parts[0].clone()),
        plan,
    };
    let revision = f.core.design().vision(&f.p).await.unwrap().revision;
    f.core
        .plans()
        .edit_design("link".into(), &f.p, revision, vec![link])
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let outdated = |cards: Vec<shadows_core::RoadmapProposal>| {
        cards.iter().find(|c| c.id == card.id).unwrap().outdated
    };
    assert!(
        !outdated(f.proposals().await),
        "a plan link is not a part change"
    );
    let part = f
        .core
        .plans()
        .design_part(&f.p, &f.parts[0])
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    let mut content = part.part.content.clone();
    content.responsibility = "Changed by hand".into();
    let put = DesignOp::PartPut {
        id: f.parts[0].clone(),
        content,
    };
    let revision = f.core.design().vision(&f.p).await.unwrap().revision;
    f.core
        .plans()
        .edit_design("part".into(), &f.p, revision, vec![put])
        .await
        .map_err(shadows_core::CoreError::from)
        .unwrap();
    assert!(
        outdated(f.proposals().await),
        "a content change outdates it"
    );
    f.close().await;
}

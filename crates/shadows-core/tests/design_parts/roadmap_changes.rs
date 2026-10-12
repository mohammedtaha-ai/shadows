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
        .design()
        .approve_roadmap_proposal("add".into(), &f.p, &added.id)
        .await
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
        .design()
        .approve_roadmap_proposal("change".into(), &f.p, &changed.id)
        .await
        .unwrap();
    let view = f.core.design().outcome(&f.p, &login.id).await.unwrap();
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
            .design()
            .approve_roadmap_proposal("blocked".into(), &f.p, &blocked.id)
            .await,
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
        .design()
        .approve_roadmap_proposal("gone".into(), &f.p, &gone.id)
        .await
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
        .design()
        .edit("hand".into(), &f.p, revision, vec![put])
        .await
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

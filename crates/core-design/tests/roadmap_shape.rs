use core_design::{Design, RoadmapAction, RoadmapPropose};
use core_design::{OutcomeContent, PartId, ProposedOutcome};

fn outcome(key: &str, parent_key: Option<&str>) -> ProposedOutcome {
    ProposedOutcome {
        key: key.into(),
        parent_key: parent_key.map(Into::into),
        parent: None,
        content: OutcomeContent {
            title: format!(" {key} "),
            intended_result: String::new(),
            acceptance: vec![],
        },
        parts: vec![PartId::generate()],
    }
}

fn first(outcomes: Vec<ProposedOutcome>) -> RoadmapPropose {
    RoadmapPropose {
        action: RoadmapAction::First,
        outcomes,
        outcome_id: None,
        replaces: None,
    }
}

#[test]
fn a_parent_cycle_is_refused() {
    let mut args = first(vec![outcome("a", Some("b")), outcome("b", Some("a"))]);
    assert!(
        Design::validate_roadmap_for_test(&mut args)
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );
}

#[test]
fn titles_are_trimmed_and_keys_must_be_unique() {
    let mut args = first(vec![outcome("a", None), outcome("b", Some("a"))]);
    Design::validate_roadmap_for_test(&mut args).unwrap();
    assert_eq!(args.outcomes[0].content.title, "a");
    let mut twice = first(vec![outcome("a", None), outcome("a", None)]);
    assert!(Design::validate_roadmap_for_test(&mut twice).is_err());
}

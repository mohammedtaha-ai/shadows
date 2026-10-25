use core_design::{Stage, StageView, TestStageRules as StageRules};

#[test]
fn approved_map_reports_new_missing_mandatory_kinds() {
    let rendered = StageRules::line(&StageView {
        stage: Stage::Structure,
        missing: vec!["billing".into()],
        drift: vec![],
    });
    assert!(
        rendered.contains("Missing mandatory parts: billing"),
        "{rendered}"
    );
    assert!(!rendered.contains("map are complete"), "{rendered}");
}

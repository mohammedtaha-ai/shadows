include!("cases.rs");

#[test]
fn every_core_error_keeps_its_http_mapping() {
    assert_eq!(Case::all().len(), 62);
    for case in Case::all() {
        let name = format!("{:?}", case.error);
        assert_eq!(case.error.code(), case.code, "{name}");
        let failure = super::Failure::from(case.error);
        assert_eq!(failure.status.as_u16(), case.status, "{name}");
        assert_eq!(failure.code, case.http_code, "{name}");
        assert_eq!(failure.message, case.http, "{name}");
        if case.code == ErrorCode::WorkflowValidationFailed {
            assert_eq!(
                failure.detail.problems,
                Some(vec!["first".into(), "second".into()])
            );
        }
        if case.code == ErrorCode::RevisionConflict {
            assert_eq!(failure.detail.current_revision, Some(2));
        }
        // Both adapters use this fixture to preserve intentional differences.
        let _ = (case.code, case.mcp_code, case.mcp);
    }
}

use serde_json::json;
use shadows_core::{AgreementContent, AgreementParty, AgreementRole, Design, PartId};

fn login() -> AgreementContent {
    AgreementContent {
        capability: "Login".into(),
        purpose: "Authenticate a user".into(),
        behavior: "No session is created after invalid credentials".into(),
        acceptance: vec!["Successful credentials return a session".into()],
        parties: vec![AgreementParty {
            part_id: PartId::from_literal("a0000000-0000-4000-8000-000000000001"),
            role: AgreementRole::Provides,
        }],
        openapi: json!({
            "openapi": "3.1.0", "info": {"title": "Login", "version": "1"},
            "paths": {"/login": {"post": {
                "x-shadows-operation-id": "b0000000-0000-4000-8000-000000000001",
                "responses": {"200": {"description": "Authenticated"}}
            }}}
        }),
    }
}

#[test]
fn agreement_content_validates_real_openapi_and_operation_identity() {
    assert!(Design::validate_agreement(&login()).is_empty());
    let mut invalid = login();
    invalid.openapi["paths"]["/login"]["post"]["responses"]["200"] = json!("wrong");
    assert!(
        Design::validate_agreement(&invalid)
            .iter()
            .any(|p| p.path.contains("responses"))
    );
    invalid = login();
    invalid.openapi["paths"]["/login"]["post"]["x-shadows-operation-id"] = json!("login");
    assert!(!Design::validate_agreement(&invalid).is_empty());
    invalid = login();
    invalid.openapi["paths"]["/other"] = invalid.openapi["paths"]["/login"].clone();
    assert!(
        Design::validate_agreement(&invalid)
            .iter()
            .any(|p| p.message.contains("duplicate"))
    );
}

#[test]
fn agreement_content_refuses_unresolved_references_and_incomplete_intent() {
    for reference in [
        "#/components/schemas/Absent",
        "https://example.com/schema",
        "file:///secret",
    ] {
        let mut invalid = login();
        invalid.openapi["paths"]["/login"]["post"]["requestBody"] = json!({"$ref": reference});
        assert!(
            Design::validate_agreement(&invalid)
                .iter()
                .any(|p| p.path.contains("$ref"))
        );
    }
    let mut incomplete = login();
    incomplete.purpose.clear();
    incomplete.acceptance.clear();
    incomplete.parties.clear();
    assert!(Design::validate_agreement(&incomplete).len() >= 3);
    let mut local = login();
    local.openapi["components"] = json!({"requestBodies": {"Login": {
        "content": {"application/json": {"schema": {"type": "object"}}}
    }}});
    local.openapi["paths"]["/login"]["post"]["requestBody"] =
        json!({"$ref": "#/components/requestBodies/Login"});
    assert!(Design::validate_agreement(&local).is_empty());
    local.openapi["components"]["requestBodies"]["Login"]["content"]["application/json"]["schema"] =
        json!({"type": "not-a-type"});
    assert!(!Design::validate_agreement(&local).is_empty());
}

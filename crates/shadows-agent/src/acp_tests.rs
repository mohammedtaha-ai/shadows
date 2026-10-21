//! ACP metadata and session-cost boundary tests.
use super::*;

#[test]
fn absent_and_empty_tool_policies_are_distinct() {
    assert!(SessionSetup::default().meta().is_none());
    let setup = SessionSetup {
        tools: Some(vec![]),
        ..Default::default()
    };
    let meta = serde_json::to_value(setup.meta().unwrap()).unwrap();
    assert_eq!(
        meta["claudeCode"]["options"]["tools"],
        serde_json::json!([])
    );
    assert_eq!(
        meta["claudeCode"]["options"]["settingSources"],
        serde_json::json!([])
    );
    assert_eq!(meta["claudeCode"]["options"]["strictMcpConfig"], true);
    let defaults = SessionSetup {
        allowed_tools: vec!["mcp__shadows__*".into()],
        ..Default::default()
    };
    let meta = serde_json::to_value(defaults.meta().unwrap()).unwrap();
    assert!(meta["claudeCode"]["options"].get("tools").is_none());
}

#[test]
fn aylul_meta_loads_only_user_settings() {
    let setup = SessionSetup {
        setting_sources: Some(vec!["user".into()]),
        ..Default::default()
    };
    let meta = serde_json::to_value(setup.meta().unwrap()).unwrap();
    assert_eq!(
        meta["claudeCode"]["options"]["settingSources"],
        serde_json::json!(["user"])
    );
    assert!(meta.get("systemPrompt").is_none());
}

#[test]
fn a_server_error_mentioning_connection_closed_is_still_an_rpc_error() {
    let error = agent_client_protocol::Error::new(-32603, "connection closed by policy");
    assert!(
        matches!(rpc(error), AcpError::Rpc(message) if message == "connection closed by policy")
    );
}
#[test]
fn session_costs_are_deltas() {
    let usd = |amount| UsageCost {
        amount,
        currency: "USD".into(),
    };
    let first = cost_delta(Some(&usd(0.0)), Some(&usd(0.2))).unwrap();
    let second = cost_delta(Some(&usd(0.2)), Some(&usd(0.5))).unwrap();
    assert!((first.amount - 0.2).abs() < 1e-12);
    assert!((second.amount - 0.3).abs() < 1e-12);
    for amount in [f64::NAN, f64::INFINITY, -1.0, 0.1] {
        assert!(cost_delta(Some(&usd(0.2)), Some(&usd(amount))).is_none());
    }
    assert!(cost_delta(None, Some(&usd(0.5))).is_none());
    assert!(cost_delta(Some(&usd(0.5)), None).is_none());
    assert!(
        cost_delta(
            Some(&usd(0.0)),
            Some(&UsageCost {
                amount: 0.2,
                currency: "EUR".into()
            })
        )
        .is_none()
    );
}

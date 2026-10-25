use super::{Format, ListedModel, parse};
fn listing(format: Format, row: serde_json::Value) -> ListedModel {
    parse(
        &serde_json::to_vec(&serde_json::json!({"data":[row]})).unwrap(),
        format,
    )
    .unwrap()
    .models
    .remove(0)
}
#[test]
fn anthropic_list_reads_max_input_tokens() {
    assert_eq!(
        listing(
            Format::Anthropic,
            serde_json::json!({"id":"claude",
        "max_input_tokens":200000})
        )
        .context_window,
        Some(200000)
    );
    let page = parse(
        br#"{"data":[],"has_more":true,"last_id":"next"}"#,
        Format::Anthropic,
    )
    .unwrap();
    assert_eq!(page.after.as_deref(), Some("next"));
    assert!(parse(br#"{"data":[{"id":2}]}"#, Format::Anthropic).is_err());
    assert!(
        parse(
            br#"{"data":[{"id":"m","max_input_tokens":"bad"}]}"#,
            Format::Anthropic
        )
        .is_err()
    );
}
#[test]
fn minimax_list_has_no_windows() {
    assert_eq!(
        listing(
            Format::MiniMax,
            serde_json::json!({"id":"MiniMax-M3",
        "type":"model","created_at":"2026-10-21","context_window":1000000})
        )
        .context_window,
        None
    );
}
#[test]
fn deepseek_list_reads_context_window() {
    assert_eq!(
        listing(
            Format::DeepSeek,
            serde_json::json!({"id":"deepseek-flash",
        "context_window":1000000})
        )
        .context_window,
        Some(1000000)
    );
}
#[test]
fn moonshot_list_reads_context_length() {
    assert_eq!(
        listing(
            Format::Moonshot,
            serde_json::json!({"id":"kimi-k3",
        "context_length":1000000})
        )
        .context_window,
        Some(1000000)
    );
}
#[test]
fn openrouter_list_keeps_tool_models() {
    let page = parse(
        br#"{"data":[{"id":"tools","context_length":123456,
        "supported_parameters":["tools","temperature"]},
        {"id":"plain","supported_parameters":["temperature"]}]}"#,
        Format::OpenRouter,
    )
    .unwrap();
    assert_eq!(page.models.len(), 1);
    assert_eq!(page.models[0].id, "tools");
    assert_eq!(page.models[0].context_window, Some(123456));
}

mod fixture;

use fixture::fixture;

#[tokio::test]
async fn the_harness_list_carries_the_policy_modes() {
    let (_tmp, _storage, harness) = fixture().await;
    let list = serde_json::to_value(harness.list().await.unwrap()).unwrap();
    let kinds: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["claude-code", "codex", "aylul"]);
    for i in [0, 2] {
        let ids: Vec<_> = list[i]["modes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["acceptEdits", "auto"]);
        assert_eq!(list[i]["modes"][0]["label"], "Accept edits");
        assert_eq!(list[i]["initial_mode"], "acceptEdits");
        assert_eq!(list[i]["unattended_mode"], "auto");
    }
    assert_eq!(list[1]["modes"][0]["id"], "ask");
    assert_eq!(list[1]["modes"][1]["id"], "read-only");
    assert_eq!(list[1]["initial_mode"], "ask");
    assert!(list[1]["unattended_mode"].is_null());
    assert_eq!(list[1]["available"], false, "Codex needs its configuration");
    assert!(
        list[1]["reason"].as_str().unwrap().contains("--codex"),
        "the reason says what to configure"
    );
}

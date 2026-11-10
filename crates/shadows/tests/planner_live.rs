//! The Planner on the real Claude adapter, against MiniMax, reaching Shadows'
//! `/mcp` on a real port. CI skips these; run one with `API_KEY_MINIMAX` set
//! and `-- --ignored --test-threads=1`.

use shadows_core::ThreadEntryKind;
use shadows_core::testing::acp;

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/listening.rs"]
mod listening;
#[path = "fixtures/live.rs"]
mod live;
#[path = "fixtures/plan.rs"]
mod plan;

/// The Planner lists the project's real files with `project_files` and
/// names the PHP files the folder holds.
#[tokio::test]
#[ignore = "live: needs API_KEY_MINIMAX"]
async fn live_the_planner_lists_the_real_files() {
    let live = live::live_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), live.adapter.clone()).await;
    let project = l.app.core.projects().get(&l.app.project).await.unwrap();
    let root = std::path::PathBuf::from(project.directory.unwrap());
    std::fs::write(root.join("composer.json"), r#"{"name":"acme/shop"}"#).unwrap();
    std::fs::create_dir_all(root.join("app/Models")).unwrap();
    std::fs::create_dir_all(root.join("routes")).unwrap();
    std::fs::write(root.join("app/Models/Customer.php"), "<?php").unwrap();
    std::fs::write(root.join("routes/web.php"), "<?php").unwrap();
    let done = live::planner_turn(
        &l.app,
        "live-files",
        "List this project's files with project_files, entering the folders it shows, \
         and answer with the paths of its PHP files only.",
    )
    .await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let entries = app::entries(&l.app).await;
    let called = entries
        .iter()
        .any(|e| e.kind == ThreadEntryKind::ToolCall && format!("{e:?}").contains("project_files"));
    assert!(called, "project_files was not called: {entries:?}");
    let reply = app::last_agent_entry(&l.app).await.body;
    println!("live files reply: {reply}");
    assert!(reply.contains("app/Models/Customer.php"), "{reply}");
    assert!(reply.contains("routes/web.php"), "{reply}");
}

/// The Planner writes a task's contract with plan_edit: the file, the
/// function's inputs and return type, and its behavior as sentences, not code.
#[tokio::test]
#[ignore = "live: needs API_KEY_MINIMAX"]
async fn live_the_planner_writes_a_contract_without_a_function_body() {
    let live = live::live_adapter();
    let tmp = tempfile::tempdir().unwrap();
    let l = listening::listening_on(tmp.path(), live.adapter.clone()).await;
    let workflow = plan::draft(&l.app).await.workflow_id;
    let prompt = format!(
        "Edit the draft plan {workflow} (it is at revision 0). Add task T1, 'Cart total': it \
         creates src/cart.rs with one function, cart_total_cents, which takes the item prices in \
         cents and returns their sum. Its one acceptance item: an empty cart totals 0. Write it \
         with plan_edit, with its contract."
    );
    let done = live::planner_turn(&l.app, "live-contract", &prompt).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    let (_, read) = app::call(&l.app, "GET", &format!("/api/workflows/{workflow}"), None).await;
    let contract = &read["tasks"][0]["contract"];
    println!("live contract: {contract}");
    assert_eq!(contract["files"][0]["path"], "src/cart.rs", "{read}");
    let symbol = &contract["symbols"][0];
    assert_eq!(symbol["name"], "cart_total_cents");
    assert!(!symbol["inputs"].as_array().unwrap().is_empty());
    assert!(
        symbol["returns"]["type_name"]
            .as_str()
            .is_some_and(|t| !t.is_empty())
    );
    let behavior = symbol["behavior"].as_array().unwrap();
    assert!(!behavior.is_empty());
    // Sentences, not code.
    assert!(
        behavior
            .iter()
            .all(|line| !line.as_str().unwrap().contains('{')),
        "{behavior:?}"
    );
}

//! A project switches library skills and MCP servers, and its `CLAUDE.md`, on
//! for its Planner. A Claude Planner opening carries what is switched on at
//! that moment: skills as one plugin folder for the thread, servers after
//! Shadows', and the file appended to the prompt. `fake_acp`'s `report` and
//! `report-mcp` show what the session opened with.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use shadows_core::testing::{SessionsConfig, Token, acp};
use shadows_core::{AylulConfig, ExtensionKind, McpInput, ThreadId};

#[path = "fixtures/app.rs"]
mod app;

use app::{
    App, create_thread, default_settings, fresh_command, last_agent_entry_on, shut_down_app,
    start_and_finish_on,
};

const ENV_VALUE: &str = "switch-env-secret";

async fn switch_app() -> App {
    let tmp = tempfile::tempdir().unwrap();
    let config = SessionsConfig {
        mcp_url: Some(acp::MCP_URL.into()),
        aylul: Some(AylulConfig {
            home_root: tmp.path().join("aylul"),
            project_slug: "aylul".into(),
        }),
        ..acp::test_config()
    };
    app::test_app_with(tmp.path(), config, acp::MCP_URL)
        .await
        .owning(tmp)
}

/// The project's directory, where its sessions open.
async fn dir(app: &App) -> PathBuf {
    let project = app.core.projects().get(&app.project).await.unwrap();
    PathBuf::from(project.directory.unwrap())
}

/// A skill named `hello` in the library; answers its id.
async fn add_skill(app: &App) -> String {
    let source = dir(app).await.join("source-hello");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: hello\ndescription: Test greeting\n---\nsay hi\n",
    )
    .unwrap();
    app.core
        .extensions()
        .add_extension(fresh_command(), ExtensionKind::Skill, &source)
        .await
        .unwrap()
        .id
}

/// A plugin in the library; answers its id.
async fn add_plugin(app: &App) -> String {
    let source = dir(app).await.join("source-plugin");
    std::fs::create_dir_all(source.join(".claude-plugin")).unwrap();
    std::fs::create_dir_all(source.join("commands")).unwrap();
    std::fs::write(
        source.join(".claude-plugin/plugin.json"),
        json!({"name":"greeter","description":"Test greeting","version":"1.0.0"}).to_string(),
    )
    .unwrap();
    std::fs::write(
        source.join("commands/hello.md"),
        "---\ndescription: Test greeting\n---\nsay hi\n",
    )
    .unwrap();
    app.core
        .extensions()
        .add_extension(fresh_command(), ExtensionKind::Plugin, &source)
        .await
        .unwrap()
        .id
}

/// A stdio server named `docs` with one env value; answers its id.
async fn add_server(app: &App) -> String {
    let input: McpInput = serde_json::from_value(json!({"name":"docs","transport":{
        "type":"stdio","command":"node","args":[],
        "env":[{"name":"TOKEN","value":ENV_VALUE}]
    }}))
    .unwrap();
    app.core
        .extensions()
        .add_mcp(fresh_command(), input)
        .await
        .unwrap()
        .id
}

async fn switch(app: &App, kind: ExtensionKind, id: &str, enabled: bool) {
    app.core
        .extensions()
        .set_project_extension(fresh_command(), &app.project, kind, id, enabled)
        .await
        .unwrap();
}

async fn claude_md(app: &App, enabled: bool) {
    app.core
        .projects()
        .set_planner_claude_md(fresh_command(), &app.project, enabled)
        .await
        .unwrap();
}

async fn reply_on(app: &App, thread: &str, prompt: &str) -> Value {
    let done = start_and_finish_on(app, thread, prompt, default_settings()).await;
    assert_eq!(done.status_kind, "Completed", "{done:?}");
    serde_json::from_str(&last_agent_entry_on(app, thread).await.body).unwrap()
}

async fn report(app: &App, thread: &str) -> Value {
    reply_on(app, thread, "report").await
}

async fn servers(app: &App, thread: &str) -> Vec<String> {
    reply_on(app, thread, "report-mcp")
        .await
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect()
}

async fn new_thread(app: &App) -> String {
    let thread = create_thread(
        app,
        json!({"command_id": fresh_command(), "title": "Another"}),
    )
    .await;
    thread["id"].as_str().unwrap().to_string()
}

async fn skills_folder(app: &App, thread: &str) -> PathBuf {
    dir(app).await.join("aylul/planner-skills").join(thread)
}

fn as_before() -> Value {
    json!({
        "allowedTools": ["mcp__shadows__*"],
        "tools": [
            "Read","Grep","Glob","WebSearch","WebFetch","ToolSearch","Agent","TodoWrite","Bash"
        ],
        "settingSources": [],
        "strictMcpConfig": true
    })
}

#[tokio::test]
async fn with_nothing_switched_on_the_planner_opens_as_before() {
    let app = switch_app().await;
    add_skill(&app).await;
    add_server(&app).await;
    let thread = app.thread.as_str().to_string();
    let r = report(&app, &thread).await;
    assert_eq!(r["options"], as_before());
    assert_eq!(servers(&app, &thread).await, ["shadows"]);
    assert!(!skills_folder(&app, &thread).await.exists());
    shut_down_app(app).await;
}

#[tokio::test]
async fn a_switched_on_skill_reaches_the_planner_as_a_plugin() {
    let app = switch_app().await;
    let skill = add_skill(&app).await;
    switch(&app, ExtensionKind::Skill, &skill, true).await;
    let thread = app.thread.as_str().to_string();
    let r = report(&app, &thread).await;
    let plugins = r["options"]["plugins"].as_array().expect("plugins");
    assert_eq!(plugins.len(), 1, "{r}");
    let folder = Path::new(
        plugins[0]
            .as_str()
            .or_else(|| plugins[0]["path"].as_str())
            .unwrap(),
    )
    .to_path_buf();
    // Windows may name the temp folder in its short form on one side only.
    assert_eq!(
        folder.canonicalize().unwrap(),
        skills_folder(&app, &thread).await.canonicalize().unwrap()
    );
    assert!(folder.join(".claude-plugin/plugin.json").is_file());
    assert!(folder.join("skills/hello/SKILL.md").is_file());
    let tools = r["options"]["tools"].as_array().unwrap();
    assert_eq!(tools.last().unwrap(), "Skill");
    assert_eq!(r["options"]["skills"], json!(["shadows-skills:hello"]));
    app.sessions
        .close_session(&ThreadId::from_literal(&thread))
        .await
        .unwrap();
    assert!(!folder.exists());
    shut_down_app(app).await;
}

#[tokio::test]
async fn a_switched_on_server_reaches_the_planner_after_shadows() {
    let app = switch_app().await;
    let server = add_server(&app).await;
    switch(&app, ExtensionKind::Mcp, &server, true).await;
    let listed = reply_on(&app, app.thread.as_str(), "report-mcp").await;
    let list = listed.as_array().unwrap();
    let names: Vec<_> = list.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["shadows", "docs"]);
    assert_eq!(list[1]["env"][0]["name"], "TOKEN");
    assert_eq!(
        list[1]["env"][0]["value_hash"],
        Token::hash_token(ENV_VALUE)
    );
    shut_down_app(app).await;
}

#[tokio::test]
async fn a_switched_off_item_and_a_library_plugin_never_reach_the_planner() {
    let app = switch_app().await;
    let skill = add_skill(&app).await;
    let server = add_server(&app).await;
    let plugin = add_plugin(&app).await;
    switch(&app, ExtensionKind::Skill, &skill, true).await;
    switch(&app, ExtensionKind::Mcp, &server, true).await;
    switch(&app, ExtensionKind::Skill, &skill, false).await;
    switch(&app, ExtensionKind::Mcp, &server, false).await;
    assert!(
        app.core
            .extensions()
            .set_project_extension(
                fresh_command(),
                &app.project,
                ExtensionKind::Plugin,
                &plugin,
                true
            )
            .await
            .is_err()
    );
    let thread = app.thread.as_str().to_string();
    assert_eq!(report(&app, &thread).await["options"], as_before());
    assert_eq!(servers(&app, &thread).await, ["shadows"]);
    shut_down_app(app).await;
}

#[tokio::test]
async fn the_claude_md_switch_appends_the_file() {
    let app = switch_app().await;
    let text = "Build with cargo. Keep tests close.";
    std::fs::write(dir(&app).await.join("CLAUDE.md"), text).unwrap();
    claude_md(&app, true).await;
    let on = report(&app, app.thread.as_str()).await;
    let append = on["append"].as_str().unwrap();
    assert!(
        append.ends_with(&format!("\n\n## The project's CLAUDE.md\n\n{text}")),
        "{append}"
    );
    claude_md(&app, false).await;
    let other = new_thread(&app).await;
    let off = report(&app, &other).await;
    assert!(!off["append"].as_str().unwrap().contains(text));
    shut_down_app(app).await;
}

#[tokio::test]
async fn a_claude_md_that_is_too_large_or_a_symlink_is_skipped() {
    let app = switch_app().await;
    let path = dir(&app).await.join("CLAUDE.md");
    std::fs::write(&path, "x".repeat(65_537)).unwrap();
    claude_md(&app, true).await;
    let r = report(&app, app.thread.as_str()).await;
    assert!(
        !r["append"]
            .as_str()
            .unwrap()
            .contains("## The project's CLAUDE.md")
    );
    #[cfg(unix)]
    {
        std::fs::remove_file(&path).unwrap();
        let target = dir(&app).await.join("real.md");
        std::fs::write(&target, "linked text").unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        let other = new_thread(&app).await;
        let r = report(&app, &other).await;
        let append = r["append"].as_str().unwrap();
        assert!(!append.contains("linked text"), "{append}");
    }
    shut_down_app(app).await;
}

#[tokio::test]
async fn a_switch_change_reaches_the_next_opening_only() {
    let app = switch_app().await;
    let server = add_server(&app).await;
    let thread = app.thread.as_str().to_string();
    assert_eq!(servers(&app, &thread).await, ["shadows"]);
    switch(&app, ExtensionKind::Mcp, &server, true).await;
    assert_eq!(servers(&app, &thread).await, ["shadows"]);
    let other = new_thread(&app).await;
    assert_eq!(servers(&app, &other).await, ["shadows", "docs"]);
    shut_down_app(app).await;
}

const UNKNOWN_PROJECT: &str = "00000000-0000-4000-8000-000000000000";

#[tokio::test]
async fn the_routes_list_and_switch_a_project_s_items() {
    let app = switch_app().await;
    let skill = add_skill(&app).await;
    let server = add_server(&app).await;
    let root = format!("/api/projects/{}/planner-extensions", app.project.as_str());

    let (status, view) = app::call(&app, "GET", &root, None).await;
    assert_eq!(status, 200, "{view}");
    assert_eq!(view["skills"][0]["id"], skill.as_str());
    assert_eq!(view["skills"][0]["enabled"], false);
    assert_eq!(view["mcp"][0]["id"], server.as_str());
    assert_eq!(view["plugins"], json!([]));

    let body = json!({"command_id": "switch-skill", "enabled": true});
    let path = format!("{root}/skill/{skill}");
    let (status, item) = app::call(&app, "PUT", &path, Some(body.clone())).await;
    assert_eq!(status, 200, "{item}");
    assert_eq!(item["id"], skill.as_str());
    assert_eq!(item["enabled"], true);
    let (status, replay) = app::call(&app, "PUT", &path, Some(body)).await;
    assert_eq!((status, replay), (200, item));

    let (status, item) = app::call(
        &app,
        "PUT",
        &format!("{root}/mcp/{server}"),
        Some(json!({"command_id": "switch-server", "enabled": true})),
    )
    .await;
    assert_eq!(status, 200, "{item}");
    assert_eq!(item["enabled"], true);
    assert_eq!(item["transport"]["type"], "stdio", "{item}");

    let (_, view) = app::call(&app, "GET", &root, None).await;
    assert_eq!(view["skills"][0]["enabled"], true);
    assert_eq!(view["mcp"][0]["enabled"], true);

    let (status, conflict) = app::call(
        &app,
        "PUT",
        &path,
        Some(json!({"command_id": "switch-skill", "enabled": false})),
    )
    .await;
    assert_eq!(status, 409, "{conflict}");
    assert_eq!(conflict["code"], "COMMAND_CONFLICT");
    shut_down_app(app).await;
}

#[tokio::test]
async fn the_routes_refuse_a_plugin_and_an_unknown_project() {
    let app = switch_app().await;
    let plugin = add_plugin(&app).await;
    let skill = add_skill(&app).await;
    let toggle = || Some(json!({"command_id": fresh_command(), "enabled": true}));

    let path = format!(
        "/api/projects/{}/planner-extensions/plugin/{plugin}",
        app.project.as_str()
    );
    let (status, refused) = app::call(&app, "PUT", &path, toggle()).await;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["code"], "PROFILE_INVALID");

    let unknown = format!("/api/projects/{UNKNOWN_PROJECT}/planner-extensions");
    let (status, missing) = app::call(&app, "GET", &unknown, None).await;
    assert_eq!(status, 404, "{missing}");
    assert_eq!(missing["code"], "EXTENSION_NOT_FOUND");
    let (status, missing) =
        app::call(&app, "PUT", &format!("{unknown}/skill/{skill}"), toggle()).await;
    assert_eq!(status, 404, "{missing}");
    assert_eq!(missing["code"], "EXTENSION_NOT_FOUND");

    let (status, missing) = app::call(
        &app,
        "PUT",
        &format!("/api/projects/{UNKNOWN_PROJECT}/planner-claude-md"),
        toggle(),
    )
    .await;
    assert_eq!(status, 404, "{missing}");
    assert_eq!(missing["code"], "INVALID_COMMAND");
    shut_down_app(app).await;
}

#[tokio::test]
async fn the_claude_md_route_sets_the_project() {
    let app = switch_app().await;
    let path = format!("/api/projects/{}/planner-claude-md", app.project.as_str());
    let (status, project) = app::call(
        &app,
        "PUT",
        &path,
        Some(json!({"command_id": "claude-md-on", "enabled": true})),
    )
    .await;
    assert_eq!(status, 200, "{project}");
    assert_eq!(project["id"], app.project.as_str());
    assert_eq!(project["planner_reads_claude_md"], true);

    let (status, projects) = app::call(&app, "GET", "/api/projects", None).await;
    assert_eq!(status, 200);
    let listed = projects
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == app.project.as_str())
        .unwrap();
    assert_eq!(listed["planner_reads_claude_md"], true);
    shut_down_app(app).await;
}

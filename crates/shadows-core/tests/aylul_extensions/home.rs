//! Per-agent projection acceptance against real home files.
use super::*;

#[tokio::test]
async fn opening_projects_only_enabled_extensions() {
    let w = world().await;
    w.core
        .profiles()
        .create_agent("b".into(), agent("Other"))
        .await
        .unwrap();
    for (kind, name) in [
        (ExtensionKind::Skill, "demo-skill"),
        (ExtensionKind::Plugin, "demo-plugin"),
    ] {
        let src = source(&w, kind, name);
        let item = w
            .core
            .profiles()
            .add_extension(name.into(), kind, &src)
            .await
            .unwrap();
        w.core
            .profiles()
            .set_agent_extension(format!("enable-{name}"), "aylul", kind, &item.id, true)
            .await
            .unwrap();
    }
    w.core.harness().open_session(&w.thread).await.unwrap();
    let home = w._tmp.path().join("home/profiles/aylul");
    assert!(home.join("skills/demo-skill/SKILL.md").exists());
    let settings: Value =
        serde_json::from_slice(&std::fs::read(home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        settings["enabledPlugins"]["demo-plugin@aylul-managed"],
        true
    );
    assert!(
        !w._tmp
            .path()
            .join("home/profiles/other/skills/demo-skill")
            .exists()
    );
}

#[tokio::test]
async fn opening_removes_disabled_cached_extensions() {
    let w = world().await;
    let src = source(&w, ExtensionKind::Plugin, "cached-plugin");
    let item = w
        .core
        .profiles()
        .add_extension("import".into(), ExtensionKind::Plugin, &src)
        .await
        .unwrap();
    w.core
        .profiles()
        .set_agent_extension("on".into(), "aylul", ExtensionKind::Plugin, &item.id, true)
        .await
        .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    w.sessions.terminate(&w.thread).await.unwrap();
    w.core
        .profiles()
        .set_agent_extension(
            "off".into(),
            "aylul",
            ExtensionKind::Plugin,
            &item.id,
            false,
        )
        .await
        .unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    let home = w._tmp.path().join("home/profiles/aylul");
    let settings: Value =
        serde_json::from_slice(&std::fs::read(home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        settings["enabledPlugins"]["cached-plugin@aylul-managed"],
        false
    );
    w.sessions.terminate(&w.thread).await.unwrap();
    let mut tampered = settings;
    tampered["enabledPlugins"]["cached-plugin@aylul-managed"] = true.into();
    tampered["enabledPlugins"]["unrelated@user-market"] = true.into();
    std::fs::write(
        home.join("settings.json"),
        serde_json::to_vec(&tampered).unwrap(),
    )
    .unwrap();
    let cache = home.join("plugins/cache/aylul-managed/cached-plugin/1.0.0");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(cache.join("kept"), "cached").unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    let settings: Value =
        serde_json::from_slice(&std::fs::read(home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(
        settings["enabledPlugins"]["cached-plugin@aylul-managed"],
        false
    );
    assert_eq!(settings["enabledPlugins"]["unrelated@user-market"], true);
    assert!(cache.join("kept").exists());
}

#[tokio::test]
async fn extension_projection_is_atomic() {
    let w = world().await;
    let src = source(&w, ExtensionKind::Skill, "collision");
    let i = w
        .core
        .profiles()
        .add_extension("i".into(), ExtensionKind::Skill, &src)
        .await
        .unwrap();
    w.core
        .profiles()
        .set_agent_extension("on".into(), "aylul", ExtensionKind::Skill, &i.id, true)
        .await
        .unwrap();
    let home = w._tmp.path().join("home/profiles/aylul");
    std::fs::create_dir_all(home.join("skills/collision")).unwrap();
    let settings = std::fs::read(home.join("settings.json")).unwrap();
    assert!(w.core.harness().open_session(&w.thread).await.is_err());
    assert_eq!(w.sessions.live_count().await, 0);
    assert_eq!(std::fs::read(home.join("settings.json")).unwrap(), settings);
    std::fs::remove_dir(home.join("skills/collision")).unwrap();
    std::fs::create_dir(home.join(".aylul-extensions.json")).unwrap();
    assert!(w.core.harness().open_session(&w.thread).await.is_err());
    assert_eq!(w.sessions.live_count().await, 0);
    assert_eq!(std::fs::read(home.join("settings.json")).unwrap(), settings);
    assert!(!home.join("skills/collision").exists());
}

#[tokio::test]
async fn parallel_projection_preserves_settings() {
    let w = world().await;
    let source = source(&w, ExtensionKind::Skill, "parallel-skill");
    let item = w
        .core
        .profiles()
        .add_extension("add".into(), ExtensionKind::Skill, &source)
        .await
        .unwrap();
    w.core
        .profiles()
        .set_agent_extension("on".into(), "aylul", ExtensionKind::Skill, &item.id, true)
        .await
        .unwrap();
    let project = w.core.profiles().assistant_project().await.unwrap();
    let second = w
        .core
        .threads()
        .create(
            "second".into(),
            &project,
            "Parallel",
            Some("aylul"),
            Some("aylul"),
        )
        .await
        .unwrap()
        .id;
    let home = w._tmp.path().join("home/profiles/aylul");
    let mut settings: Value =
        serde_json::from_slice(&std::fs::read(home.join("settings.json")).unwrap()).unwrap();
    settings["permissions"] = json!({"allow":["Read"]});
    std::fs::write(home.join("settings.json"), settings.to_string()).unwrap();
    let (a, b, toggled) = tokio::join!(
        w.core.harness().open_session(&w.thread),
        w.core.harness().open_session(&second),
        w.core.profiles().set_agent_extension(
            "off".into(),
            "aylul",
            ExtensionKind::Skill,
            &item.id,
            false
        )
    );
    a.unwrap();
    b.unwrap();
    toggled.unwrap();
    let settings: Value =
        serde_json::from_slice(&std::fs::read(home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(settings["permissions"], json!({"allow":["Read"]}));
    if home.join("skills/parallel-skill/SKILL.md").exists() {
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(home.join(".aylul-extensions.json")).unwrap())
                .unwrap();
        assert!(
            manifest["skills"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n == "parallel-skill")
        );
    }
    w.sessions.terminate(&w.thread).await.unwrap();
    w.sessions.terminate(&second).await.unwrap();
    w.core.harness().open_session(&w.thread).await.unwrap();
    assert!(!home.join("skills/parallel-skill").exists());
}

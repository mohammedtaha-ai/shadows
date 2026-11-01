use super::{
    binding::{BindingView, ConversationTarget},
    core_client::Client,
    watcher::{PresenceWatcher, Timing, Update},
    watcher_fixture::*,
};
use crate::core_connection::ReadyConnection;
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
fn binding(target: bool) -> BindingView {
    BindingView {
        revision: 1,
        generation: 1,
        configuration_id: Some(CONFIG.into()),
        agent: Some("agent-a".into()),
        target: target.then(|| ConversationTarget {
            project_id: PROJECT.into(),
            thread_id: THREAD_A.into(),
        }),
        previews: true,
    }
}
fn ready(api: &Fixture) -> ReadyConnection {
    ReadyConnection {
        origin: api.origin.clone(),
        configuration_id: CONFIG.into(),
        runtime_id: RUNTIME.into(),
    }
}
fn watch(
    api: &Fixture,
    target: bool,
) -> (Arc<PresenceWatcher>, UnboundedReceiver<Update>, BindingView) {
    let (tx, rx) = unbounded_channel();
    let watcher = PresenceWatcher::with_timing(
        Arc::new(move |update| {
            let _ = tx.send(update);
        }),
        Timing::test(),
    );
    let binding = binding(target);
    watcher.sync(binding.clone(), Some(ready(api)));
    (watcher, rx, binding)
}
async fn colour(rx: &mut UnboundedReceiver<Update>, expected: Option<&str>) -> Update {
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            let mut update = rx.recv().await.unwrap();
            let matches = update.appearance.as_ref().map(|v| v.color.as_str()) == expected;
            update.acknowledge();
            if matches {
                return update;
            }
        }
    })
    .await
    .expect("appearance publication deadline")
}
#[tokio::test]
async fn appearance_check_copies_opaque_outfit_and_defaults_absent() {
    use axum::{routing::get, Json, Router};
    use serde_json::json;

    for outfit in [
        None,
        Some(""),
        Some("opaque(🌿)"),
        Some("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
    ] {
        let mut agent = json!({"slug":"agent-a","provider":"provider-a","deleted":false,"character_color":"blue"});
        if let Some(value) = outfit {
            agent["outfit"] = value.into();
        }
        let app = Router::new()
            .route("/api/runtime", get(|| async { Json(json!({"runtime_id":super::watcher_fixture::RUNTIME,"configuration_id":super::watcher_fixture::CONFIG})) }))
            .route("/api/aylul/providers", get(|| async { Json(json!([{"slug":"provider-a","key_set":true}])) }))
            .route("/api/aylul/agents", get(move || { let agent = agent.clone(); async move { Json(json!({"project_id":super::watcher_fixture::PROJECT,"agents":[agent]})) } }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = Client::new(
            ReadyConnection {
                origin,
                configuration_id: super::watcher_fixture::CONFIG.into(),
                runtime_id: super::watcher_fixture::RUNTIME.into(),
            },
            Duration::from_secs(1),
        )
        .unwrap();
        let appearance = client
            .validate_appearance(&binding(false))
            .await
            .unwrap()
            .unwrap();
        let value = serde_json::to_value(appearance).unwrap();
        assert_eq!(value.get("outfit").and_then(|v| v.as_str()), outfit);
        server.abort();
    }
}
#[tokio::test]
async fn panel_metadata_is_bounded_and_marks_only_the_current_agent() {
    use axum::{routing::get, Json, Router};
    use serde_json::json;
    let agents: Vec<_> = (0..70)
        .map(|index| {
            json!({
                "slug":format!("agent-{index}"), "provider":"provider-a", "deleted":index==0,
                "name":"名".repeat(140), "description":"🌿".repeat(220),
                "character_color":"blue", "outfit":"headphones"
            })
        })
        .collect();
    let app = Router::new()
        .route(
            "/api/runtime",
            get(|| async { Json(json!({"runtime_id":RUNTIME,"configuration_id":CONFIG})) }),
        )
        .route(
            "/api/aylul/agents",
            get(move || {
                let agents = agents.clone();
                async move { Json(json!({"project_id":PROJECT,"agents":agents})) }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = Client::new(
        ReadyConnection {
            origin,
            configuration_id: CONFIG.into(),
            runtime_id: RUNTIME.into(),
        },
        Duration::from_secs(1),
    )
    .unwrap();
    let mut view = binding(false);
    view.agent = Some("agent-1".into());
    let agents = client.panel_agents(&view).await.unwrap();
    assert_eq!(agents.len(), 64);
    assert_eq!(agents[0].slug, "agent-1");
    assert!(agents[0].bound);
    assert_eq!(agents.iter().filter(|agent| agent.bound).count(), 1);
    assert_eq!(agents[0].name.chars().count(), 128);
    assert_eq!(agents[0].description.chars().count(), 200);
    assert_eq!(agents[0].color, "blue");
    assert_eq!(agents[0].outfit, "headphones");
    server.abort();
}

#[tokio::test]
async fn appearance_check_copies_opaque_colour() {
    let api = Fixture::new().await;
    let client = Client::new(ready(&api), Duration::from_secs(1)).unwrap();
    for value in [
        None,
        Some("blue"),
        Some("teal"),
        Some(""),
        Some("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
    ] {
        *api.data.colour.lock().unwrap() = value.map(str::to_owned);
        assert_eq!(
            client
                .validate_appearance(&binding(false))
                .await
                .unwrap()
                .map(|v| v.color),
            value.map(str::to_owned)
        );
    }
    assert!(api
        .data
        .requests
        .lock()
        .unwrap()
        .iter()
        .all(|v| v.starts_with("GET ")));
}
#[tokio::test]
async fn appearance_selection_and_liveness_publish_colour() {
    for target in [false, true] {
        let api = Fixture::new().await;
        *api.data.colour.lock().unwrap() = Some("blue".into());
        let (watcher, mut rx, _) = watch(&api, target);
        colour(&mut rx, Some("blue")).await;
        *api.data.colour.lock().unwrap() = Some("teal".into());
        colour(&mut rx, Some("teal")).await;
        watcher.stop();
    }
}
#[tokio::test]
async fn appearance_unbound_and_unavailable_are_absent() {
    for problem in 0..4 {
        let api = Fixture::new().await;
        *api.data.colour.lock().unwrap() = Some("blue".into());
        let (watcher, mut rx, _) = watch(&api, false);
        colour(&mut rx, Some("blue")).await;
        match problem {
            0 => api.data.deleted.store(true, Ordering::SeqCst),
            1 => api.data.key.store(false, Ordering::SeqCst),
            2 => api.data.wrong_runtime.store(true, Ordering::SeqCst),
            _ => api.data.offline.store(true, Ordering::SeqCst),
        }
        let update = colour(&mut rx, None).await;
        assert_ne!(update.projection.status, "Ready");
        watcher.stop();
    }
}
#[tokio::test]
async fn appearance_refresh_waits_for_recheck_and_publication() {
    let api = Fixture::new().await;
    *api.data.colour.lock().unwrap() = Some("blue".into());
    let (watcher, mut rx, binding) = watch(&api, true);
    colour(&mut rx, Some("blue")).await;
    *api.data.colour.lock().unwrap() = Some("teal".into());
    let task = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.refresh_appearance(&binding).await }
    });
    let mut publication = loop {
        let update = rx.recv().await.unwrap();
        if update.has_acknowledgement() {
            break update;
        }
    };
    assert!(!task.is_finished(), "refresh must wait for UI publication");
    assert_eq!(publication.appearance.as_ref().unwrap().color, "teal");
    assert_ne!(publication.projection.expression, "success");
    publication.acknowledge();
    assert_eq!(
        serde_json::to_value(task.await.unwrap().unwrap()).unwrap(),
        serde_json::json!({"ok":true})
    );
    assert_eq!(
        api.data
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.contains("/api/subscribe"))
            .count(),
        1
    );
    watcher.stop();
}
#[tokio::test]
async fn appearance_refresh_unbound_republishes_without_core() {
    let api = Fixture::new().await;
    let (tx, mut rx) = unbounded_channel();
    let watcher = PresenceWatcher::with_timing(
        Arc::new(move |update| {
            let _ = tx.send(update);
        }),
        Timing::test(),
    );
    let mut binding = binding(false);
    binding.agent = None;
    watcher.sync(binding.clone(), None);
    let task = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.refresh_appearance(&binding).await }
    });
    loop {
        let mut update = rx.recv().await.unwrap();
        if update.has_acknowledgement() {
            assert!(update.appearance.is_none());
            update.acknowledge();
            break;
        }
    }
    assert!(task.await.unwrap().unwrap().ok);
    assert!(api.data.requests.lock().unwrap().is_empty());
    watcher.stop();
}
#[tokio::test]
async fn appearance_refresh_cannot_publish_a_superseded_binding() {
    for change in 0..3 {
        let api = Fixture::new().await;
        *api.data.colour.lock().unwrap() = Some("blue".into());
        let (watcher, mut rx, old) = watch(&api, false);
        colour(&mut rx, Some("blue")).await;
        api.data.metadata_delay.store(150, Ordering::SeqCst);
        let task = tokio::spawn({
            let watcher = watcher.clone();
            let old = old.clone();
            async move { watcher.refresh_appearance(&old).await }
        });
        tokio::time::sleep(Duration::from_millis(30)).await;
        let mut new = old;
        let mut connection = ready(&api);
        match change {
            0 => {
                new.generation += 1;
                new.agent = Some("agent-b".into());
            }
            1 => {
                new.generation += 1;
                new.configuration_id = Some("new-configuration".into());
                connection.configuration_id = "new-configuration".into();
            }
            _ => {
                connection.runtime_id = "new-runtime".into();
            }
        }
        watcher.sync(new, Some(connection));
        let update = colour(&mut rx, if change == 0 { Some("pink") } else { None }).await;
        assert!(update.is_current());
        assert!(task.await.unwrap().is_err());
        watcher.stop();
    }
}
#[tokio::test]
async fn appearance_refresh_failure_is_fixed_and_polling_recovers() {
    let api = Fixture::new().await;
    let (watcher, mut rx, binding) = watch(&api, false);
    colour(&mut rx, None).await;
    api.data.offline.store(true, Ordering::SeqCst);
    let task = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.refresh_appearance(&binding).await }
    });
    loop {
        let mut update = rx.recv().await.unwrap();
        if update.has_acknowledgement() {
            assert!(update.appearance.is_none());
            update.acknowledge();
            break;
        }
    }
    assert_eq!(
        task.await.unwrap().unwrap_err(),
        "companion_core_unavailable"
    );
    api.data.offline.store(false, Ordering::SeqCst);
    *api.data.colour.lock().unwrap() = Some("blue".into());
    colour(&mut rx, Some("blue")).await;
    watcher.stop();
}

#[tokio::test]
async fn appearance_refresh_serializes_after_an_older_periodic_read() {
    let api = Fixture::new().await;
    *api.data.colour.lock().unwrap() = Some("blue".into());
    let (watcher, mut rx, binding) = watch(&api, false);
    colour(&mut rx, Some("blue")).await;
    api.data.metadata_delay.store(150, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(2), async {
        while api.data.metadata_delay.load(Ordering::SeqCst) != 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    *api.data.colour.lock().unwrap() = Some("teal".into());
    let task = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.refresh_appearance(&binding).await }
    });
    loop {
        let mut update = rx.recv().await.unwrap();
        if update.has_acknowledgement() {
            assert_eq!(update.appearance.as_ref().unwrap().color, "teal");
            update.acknowledge();
            break;
        }
    }
    assert!(task.await.unwrap().is_ok());
    tokio::time::sleep(Duration::from_millis(100)).await;
    while let Ok(update) = rx.try_recv() {
        assert_eq!(update.appearance.as_ref().unwrap().color, "teal");
    }
    watcher.stop();
}
#[tokio::test]
async fn appearance_refresh_works_after_a_terminal_feed_stop() {
    let api = Fixture::new().await;
    api.data.wrong_runtime.store(true, Ordering::SeqCst);
    let (watcher, mut rx, binding) = watch(&api, false);
    loop {
        let update = rx.recv().await.unwrap();
        if update.projection.status.starts_with("Core changed") {
            break;
        }
    }
    api.data.wrong_runtime.store(false, Ordering::SeqCst);
    *api.data.colour.lock().unwrap() = Some("teal".into());
    let task = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.refresh_appearance(&binding).await }
    });
    loop {
        let mut update = rx.recv().await.unwrap();
        if update.has_acknowledgement() {
            assert_eq!(update.appearance.as_ref().unwrap().color, "teal");
            update.acknowledge();
            break;
        }
    }
    assert!(task.await.unwrap().is_ok());
    watcher.stop();
}

#[tokio::test]
async fn appearance_refresh_before_the_first_binding_notification_is_unbound_success() {
    let (tx, mut rx) = unbounded_channel();
    let watcher = PresenceWatcher::with_timing(
        Arc::new(move |update| {
            let _ = tx.send(update);
        }),
        Timing::test(),
    );
    let mut binding = binding(false);
    binding.agent = None;
    binding.configuration_id = None;
    let task = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.refresh_appearance(&binding).await }
    });
    loop {
        tokio::select! {
            result=rx.recv()=>{let mut update=result.unwrap();if update.has_acknowledgement(){update.acknowledge();break;}},
            _=tokio::time::sleep(Duration::from_millis(200))=>{assert!(!task.is_finished(),"initial unbound refresh was refused");}
        }
    }
    assert!(task.await.unwrap().unwrap().ok);
    watcher.stop();
}

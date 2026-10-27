use crate::aylul_env::tests::{World, request, world};
use axum::http::StatusCode;
use serde_json::{Value, json};

async fn call(w: &World, method: &str, path: &str, body: Value) -> (u16, Value) {
    let (status, text) = request(w, method, path, &body.to_string()).await;
    (
        status.as_u16(),
        serde_json::from_str(&text).unwrap_or(Value::Null),
    )
}

fn provider() -> Value {
    json!({"command_id":"provider","name":"Test","base_url":"https://example.com",
        "models":[{"id":"model-a","context_window":200000},{"id":"model-b"}]})
}

async fn seed(w: &World, name: &str) {
    if w.core.providers().providers().await.unwrap().is_empty() {
        w.core
            .providers()
            .create_provider(
                "seed-provider".into(),
                serde_json::from_value(provider()).unwrap(),
            )
            .await
            .unwrap();
    }
    w.core
        .agents()
        .create_agent(
            format!("seed-{name}"),
            shadows_core::AgentInput {
                character_color: None,
                name: name.into(),
                provider: "test-2".into(),
                default_model: "model-a".into(),
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn provider_crud_and_key_never_echoed() {
    let w = world(true, true).await;
    let (status, p) = call(&w, "POST", "/api/aylul/providers", provider()).await;
    assert_eq!(status, 201);
    assert_eq!(p["key_set"], false);
    assert_eq!(p["slug"], "test-2");
    let (status, text) = request(
        &w,
        "PUT",
        "/api/aylul/providers/test-2/key",
        r#"{"command_id":"key","value":"synthetic-route-secret"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!text.contains("synthetic-route-secret"));
    let (status, list) = call(&w, "GET", "/api/aylul/providers", Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(list[0]["key_set"], true);
    assert!(!list.to_string().contains("synthetic-route-secret"));
    let mut update = provider();
    update["command_id"] = "update".into();
    update["name"] = "Renamed".into();
    assert_eq!(
        call(&w, "PUT", "/api/aylul/providers/test-2", update)
            .await
            .1["slug"],
        "test-2"
    );
    assert_eq!(
        call(
            &w,
            "DELETE",
            "/api/aylul/providers/test-2?command_id=delete",
            Value::Null
        )
        .await
        .0,
        204
    );

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn key_body_parse_error_does_not_echo() {
    let w = world(true, true).await;
    let (status, text) = request(
        &w,
        "PUT",
        "/api/aylul/providers/test-2/key",
        r#"{"command_id":"key","value":{"secret":"synthetic-do-not-echo"}}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!text.contains("synthetic-do-not-echo"));

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn agent_crud_soft_delete() {
    let w = world(true, true).await;
    call(&w, "POST", "/api/aylul/providers", provider()).await;
    let (status, a) = call(
        &w,
        "POST",
        "/api/aylul/agents",
        json!({"command_id":"agent",
        "name":"Work","provider":"test-2","default_model":"model-a"}),
    )
    .await;
    assert_eq!(status, 201);
    assert_eq!(a["slug"], "work");
    assert_eq!(a["character_color"], "purple");
    let (status, list) = call(&w, "GET", "/api/aylul/agents", Value::Null).await;
    assert_eq!(status, 200);
    assert!(list["project_id"].is_string());
    assert_eq!(list["agents"][0]["slug"], "work");
    assert_eq!(list["agents"][0]["character_color"], "purple");
    assert_eq!(
        call(
            &w,
            "DELETE",
            "/api/aylul/agents/work?command_id=delete",
            Value::Null
        )
        .await
        .0,
        204
    );
    assert!(w.core.agents().agent("work").await.unwrap().deleted);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn provider_in_use_is_409() {
    let w = world(true, true).await;
    seed(&w, "Work").await;
    let (status, body) = call(
        &w,
        "DELETE",
        "/api/aylul/providers/test-2?command_id=delete",
        Value::Null,
    )
    .await;
    assert_eq!(status, 409);
    assert_eq!(body["code"], "PROVIDER_IN_USE");

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn env_routes_are_per_agent() {
    let w = world(true, true).await;
    seed(&w, "Work").await;
    seed(&w, "Personal").await;
    assert_eq!(
        call(
            &w,
            "PUT",
            "/api/aylul/agents/work/env/EXTRA",
            json!({"command_id":"env","value":"synthetic"})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(&w, "GET", "/api/aylul/agents/personal/env", Value::Null)
            .await
            .1,
        json!([])
    );
    assert_eq!(
        call(&w, "GET", "/api/aylul/agents/work/env", Value::Null)
            .await
            .1[0]["name"],
        "EXTRA"
    );
    assert_eq!(call(&w, "GET", "/api/aylul/env", Value::Null).await.0, 404);

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn reserved_name_is_409() {
    let w = world(true, true).await;
    seed(&w, "Work").await;
    for name in ["ANTHROPIC_BASE_URL", "ANTHROPIC_AUTH_TOKEN"] {
        let (status, body) = call(
            &w,
            "PUT",
            &format!("/api/aylul/agents/work/env/{name}"),
            json!({"command_id":name,"value":"synthetic"}),
        )
        .await;
        assert_eq!(status, 409);
        assert_eq!(body["code"], "SECRET_NAME_RESERVED");
    }

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn thread_create_with_profile() {
    let w = world(true, true).await;
    seed(&w, "Work").await;
    let project = w.core.agents().assistant_project().await.unwrap();
    let (status, t) = call(
        &w,
        "POST",
        &format!("/api/projects/{project}/threads"),
        json!({"command_id":"thread","title":"Chat","harness":"aylul","profile":"work"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(t["profile"], "work");

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn threads_listed_per_agent() {
    let w = world(true, true).await;
    seed(&w, "Work").await;
    seed(&w, "Personal").await;
    let project = w.core.agents().assistant_project().await.unwrap();
    w.core
        .threads()
        .create(
            "thread".into(),
            &project,
            "Work chat",
            Some("aylul"),
            Some("work"),
        )
        .await
        .unwrap();
    let (status, list) = call(&w, "GET", "/api/aylul/agents/work/threads", Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(list[0]["title"], "Work chat");
    assert_eq!(
        call(&w, "GET", "/api/aylul/agents/personal/threads", Value::Null)
            .await
            .1,
        json!([])
    );

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn provider_presets_route_lists_all() {
    let w = world(true, true).await;
    let (status, list) = call(&w, "GET", "/api/aylul/provider-presets", Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(list.as_array().unwrap().len(), 7);
    assert_eq!(list[0]["id"], "minimax");
    assert_eq!(list[1]["region"], "china");

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn provider_test_route_answers_outcome() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let w = world(true, true).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        socket
            .write_all(b"HTTP/1.1 401 Refused\r\nContent-Length: 0\r\n\r\n")
            .await
            .unwrap();
    });
    let (status, r) = call(
        &w,
        "POST",
        "/api/aylul/providers/test",
        json!({"base_url":url,"api_key":"test-key-not-real"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(r["reason"], "key_refused");
    server.await.unwrap();
    assert_eq!(
        call(&w, "POST", "/api/aylul/providers/missing/test", json!({}))
            .await
            .0,
        404
    );
    call(&w, "POST", "/api/aylul/providers", provider()).await;
    assert_eq!(
        call(&w, "POST", "/api/aylul/providers/test-2/test", json!({}))
            .await
            .0,
        409
    );
    assert_eq!(
        call(
            &w,
            "POST",
            "/api/aylul/providers/test",
            json!({"api_key":""})
        )
        .await
        .0,
        400
    );

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn me_round_trips() {
    let w = world(false, false).await;
    let (status, p) = call(&w, "GET", "/api/me", Value::Null).await;
    assert_eq!(status, 200);
    assert_eq!(
        p,
        json!({"name":null,"aylul_calls_me":null,"language":null,
        "theme":"system","updated_at":null})
    );
    let input = json!({"command_id":"person","name":"  Alex  ","aylul_calls_me":"Lex",
        "language":"en","theme":"dark"});
    let (status, p) = call(&w, "PATCH", "/api/me", input.clone()).await;
    assert_eq!(status, 200);
    assert_eq!(p["name"], "Alex");
    assert_eq!(p["aylul_calls_me"], "Lex");
    assert_eq!(p["language"], "en");
    assert_eq!(p["theme"], "dark");
    assert!(p["updated_at"].is_string());
    assert_eq!(call(&w, "GET", "/api/me", Value::Null).await.1, p);
    let mut changed = input.clone();
    changed["theme"] = "light".into();
    assert_eq!(call(&w, "PATCH", "/api/me", changed).await.0, 409);
    for (field, value) in [
        ("theme", "other"),
        ("name", "bad\nname"),
        ("language", "bad_tag"),
    ] {
        let mut bad = input.clone();
        bad["command_id"] = field.into();
        bad[field] = value.into();
        assert_eq!(call(&w, "PATCH", "/api/me", bad).await.0, 400);
    }

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn me_partial_updates_preserve_fields_and_refuse_empty_patches() {
    let w = world(false, false).await;
    let seed = json!({"command_id":"seed", "name":"Alex", "aylul_calls_me":"Lex", "language":"en", "theme":"dark"});
    assert_eq!(call(&w, "PATCH", "/api/me", seed.clone()).await.0, 200);
    let name_patch = json!({"command_id":"name", "name":"  Alex Smith  "});
    assert_eq!(
        call(&w, "PATCH", "/api/me", name_patch.clone()).await.0,
        200
    );
    let (status, updated) = call(
        &w,
        "PATCH",
        "/api/me",
        json!({"command_id":"theme", "theme":"light"}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(updated["name"], "Alex Smith");
    assert_eq!(updated["aylul_calls_me"], "Lex");
    assert_eq!(updated["language"], "en");
    assert_eq!(updated["theme"], "light");
    assert_eq!(call(&w, "PATCH", "/api/me", name_patch).await.1, updated);
    for changed in [
        json!({"command_id":"name", "name":"Alex Smith", "language":"en"}),
        json!({"command_id":"name", "theme":"light"}),
    ] {
        let (status, body) = call(&w, "PATCH", "/api/me", changed).await;
        assert_eq!(status, 409);
        assert_eq!(body["code"], "COMMAND_CONFLICT");
    }
    let (status, empty) = call(&w, "PATCH", "/api/me", json!({"command_id":"empty"})).await;
    assert_eq!(status, 400);
    assert_eq!(empty["code"], "PROFILE_INVALID");
    assert_eq!(
        empty["message"],
        "invalid profile: person patch has no fields"
    );
    assert_eq!(call(&w, "GET", "/api/me", Value::Null).await.1, updated);
    for (field, value, message) in [
        (
            "name",
            "bad\nname".to_owned(),
            "invalid profile: invalid person name",
        ),
        (
            "aylul_calls_me",
            "a".repeat(65),
            "invalid profile: person name is too long",
        ),
        (
            "language",
            "bad_tag".to_owned(),
            "invalid profile: invalid language tag",
        ),
    ] {
        let (status, body) = call(
            &w,
            "PATCH",
            "/api/me",
            json!({"command_id":field, field:value}),
        )
        .await;
        assert_eq!(status, 400);
        assert_eq!(body["message"], message);
    }
    assert_eq!(
        call(
            &w,
            "PATCH",
            "/api/me",
            json!({"command_id":"null-theme", "theme":null})
        )
        .await
        .0,
        400
    );
    let (status, cleared) = call(
        &w,
        "PATCH",
        "/api/me",
        json!({"command_id":"clear", "name":null,"aylul_calls_me":null,"language":null}),
    )
    .await;
    assert_eq!(status, 200);
    for field in ["name", "aylul_calls_me", "language"] {
        assert!(cleared[field].is_null());
    }
    assert_eq!(cleared["theme"], "light");
    assert_eq!(call(&w, "PUT", "/api/me", seed).await.0, 405);
    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn reserved_test_path_has_no_mutation_aliases() {
    let w = world(true, true).await;
    for method in ["PUT", "DELETE"] {
        let (status, _) = call(
            &w,
            method,
            "/api/aylul/providers/test?command_id=reserved",
            json!({}),
        )
        .await;
        assert_eq!(status, 405);
    }

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

#[tokio::test]
async fn usage_route_is_read_only() {
    let w = world(false, false).await;
    let p = w.core.person().person().await.unwrap();
    let (status, r) = call(
        &w,
        "GET",
        "/api/aylul/usage?from=2000-01-01T00:00:00Z&to=2100-01-01T00:00:00Z",
        Value::Null,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(r, json!({"by_provider":[],"by_model":[]}));
    assert_eq!(
        serde_json::to_value(w.core.person().person().await.unwrap()).unwrap(),
        serde_json::to_value(p).unwrap()
    );
    for path in [
        "/api/aylul/usage",
        "/api/aylul/usage?from=bad&to=bad",
        "/api/aylul/usage?from=2100-01-01T00:00:00Z&to=2000-01-01T00:00:00Z",
    ] {
        assert_eq!(call(&w, "GET", path, Value::Null).await.0, 400);
    }

    w.core
        .shut_down(std::time::Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

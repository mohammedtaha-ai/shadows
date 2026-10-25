//! Provider probes use a bounded synthetic HTTP peer, never a real key.
use super::*;
use shadows_core::{ProviderTest, ProviderTestOutcome};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn peer(
    status: u16,
    body: String,
    headers: &str,
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let headers = headers.to_owned();
    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            let mut buf = [0; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            request.extend_from_slice(&buf[..n]);
            if n == 0 || request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let answer = format!(
            "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\n\
            Connection: close\r\n{headers}\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(answer.as_bytes()).await;
        String::from_utf8(request).unwrap()
    });
    (url, handle)
}

fn input(url: String) -> ProviderTest {
    ProviderTest {
        preset: None,
        base_url: Some(url),
        api_key: "test-key-not-real".into(),
    }
}

async fn outcome(status: u16, body: &str) -> ProviderTestOutcome {
    let w = world().await;
    let (url, handle) = peer(status, body.into(), "").await;
    let result = w.app.providers().test_provider(input(url)).await.unwrap();
    handle.await.unwrap();
    result
}

#[tokio::test]
async fn tested_key_is_not_stored_or_echoed() {
    let w = world().await;
    let mut writes = testing::DatabaseWrites::observe(&w.storage).await.unwrap();
    let before = writes.version().await.unwrap();
    let (url, handle) = peer(400, "test-key-not-real".into(), "").await;
    let request = input(url);
    assert!(!format!("{request:?}").contains("test-key-not-real"));
    let result = w.app.providers().test_provider(request).await.unwrap();
    handle.await.unwrap();
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("test-key-not-real")
    );
    assert!(w.app.providers().providers().await.unwrap().is_empty());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secret_env")
        .fetch_one(w.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(writes.version().await.unwrap(), before);
    let echoed = outcome(200, r#"{"data":[{"id":"test-key-not-real"}]}"#).await;
    assert!(!echoed.ok);
    assert!(
        !serde_json::to_string(&echoed)
            .unwrap()
            .contains("test-key-not-real")
    );
}

#[tokio::test]
async fn probe_timeout_is_unreachable() {
    let w = world().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let start = std::time::Instant::now();
    let r = w.app.providers().test_provider(input(url)).await.unwrap();
    server.abort();
    assert_eq!(serde_json::to_value(r.reason).unwrap(), "unreachable");
    assert!(start.elapsed() < std::time::Duration::from_secs(15));
}

#[tokio::test]
async fn refused_key_reads_key_refused() {
    for status in [401, 403] {
        let r = outcome(status, "test-key-not-real").await;
        assert!(!r.ok);
        assert_eq!(serde_json::to_value(r.reason).unwrap(), "key_refused");
        assert_eq!(r.status, Some(status));
    }
}

#[tokio::test]
async fn unreachable_host_reads_unreachable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let w = world().await;
    let r = w.app.providers().test_provider(input(url)).await.unwrap();
    assert_eq!(serde_json::to_value(r.reason).unwrap(), "unreachable");
    assert_eq!(r.status, None);
    assert_eq!(
        serde_json::to_value(outcome(503, "").await.reason).unwrap(),
        "unreachable"
    );
}

#[tokio::test]
async fn odd_status_reads_unexpected() {
    let r = outcome(429, "test-key-not-real").await;
    assert_eq!(serde_json::to_value(r.reason).unwrap(), "unexpected");
    assert_eq!(r.status, Some(429));
    assert!(!r.message.unwrap().contains("test-key-not-real"));
    assert!(!outcome(200, "not json").await.ok);
}

#[tokio::test]
async fn redirect_is_not_followed() {
    let w = world().await;
    let target = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!(
        "Location: http://{}/stolen\r\n",
        target.local_addr().unwrap()
    );
    let (url, handle) = peer(302, String::new(), &location).await;
    let r = w.app.providers().test_provider(input(url)).await.unwrap();
    handle.await.unwrap();
    assert_eq!(serde_json::to_value(r.reason).unwrap(), "unexpected");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), target.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn oversized_body_is_unexpected() {
    let r = outcome(200, &"x".repeat(8 * 1024 * 1024 + 1)).await;
    assert_eq!(serde_json::to_value(r.reason).unwrap(), "unexpected");
}

#[tokio::test]
async fn custom_without_model_list_is_ok_unlisted() {
    let r = outcome(404, "").await;
    assert!(r.ok);
    assert!(!r.listed);
    assert!(r.models.is_empty());
    let r = outcome(
        200,
        r#"{"data":[{"id":"z"},{"id":"a","max_input_tokens":123456}]}"#,
    )
    .await;
    assert!(r.ok && r.listed);
    assert_eq!(r.models[0].id, "a");
    assert_eq!(r.models[0].context_window, Some(123456));
    assert_eq!(r.models[1].context_window, None);
}

#[tokio::test]
async fn saved_test_uses_the_stored_key() {
    let w = world().await;
    let core = super::threads::core(&w).await;
    let (url, handle) = peer(200, r#"{"data":[{"id":"m"}]}"#.into(), "").await;
    let mut p = provider("Provider");
    p.base_url = Some(url);
    w.app
        .providers()
        .create_provider("p".into(), p)
        .await
        .unwrap();
    w.app
        .providers()
        .set_key("k".into(), "provider", "test-key-not-real")
        .await
        .unwrap();
    let mut writes = testing::DatabaseWrites::observe(&w.storage).await.unwrap();
    let before = writes.version().await.unwrap();
    let r = core
        .providers()
        .test_saved_provider("provider", None)
        .await
        .unwrap();
    assert!(r.ok);
    assert_eq!(writes.version().await.unwrap(), before);
    let request = handle.await.unwrap().to_lowercase();
    assert!(request.contains("authorization: bearer test-key-not-real"));
    assert!(request.contains("x-api-key: test-key-not-real"));
    w.app
        .providers()
        .remove_key("r".into(), "provider")
        .await
        .unwrap();
    assert!(matches!(
        core.providers().test_saved_provider("provider", None).await,
        Err(shadows_core::ProvidersError::KeyMissing(_))
    ));
    assert!(
        core.providers()
            .test_saved_provider("absent", None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn invalid_probe_input_is_refused() {
    let w = world().await;
    for v in [
        serde_json::json!({"api_key":"test-key-not-real"}),
        serde_json::json!({"preset":"unknown","api_key":"test-key-not-real"}),
        serde_json::json!({"preset":"minimax","base_url":"https://example.test",
            "api_key":"test-key-not-real"}),
        serde_json::json!({"base_url":"http://example.test","api_key":"test-key-not-real"}),
        serde_json::json!({"preset":"minimax","api_key":""}),
    ] {
        assert!(matches!(
            w.app
                .providers()
                .test_provider(serde_json::from_value(v).unwrap())
                .await,
            Err(shadows_core::ProvidersError::InvalidProfile(_))
        ));
    }
}

#[tokio::test]
#[ignore = "uses the authorized OpenRouter and MiniMax environment keys"]
async fn live_keys_list_models() {
    let w = world().await;
    for (preset, name) in [("openrouter", "OPENROUTER"), ("minimax", "API_KEY_MINIMAX")] {
        let Ok(api_key) = std::env::var(name) else {
            continue;
        };
        let r = w
            .app
            .providers()
            .test_provider(ProviderTest {
                preset: Some(preset.into()),
                base_url: None,
                api_key,
            })
            .await
            .unwrap();
        println!(
            "{preset} ok={} reason={:?} models={}",
            r.ok,
            r.reason,
            r.models.len()
        );
        assert!(r.ok && !r.models.is_empty(), "live model listing failed");
    }
}

#[tokio::test]
async fn anthropic_pagination_stops_at_five_pages() {
    let w = world().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for page in 0..5 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = [0; 4096];
            let n = socket.read(&mut bytes).await.unwrap();
            let request = std::str::from_utf8(&bytes[..n]).unwrap();
            if page > 0 {
                assert!(request.contains(&format!("after_id=p{}", page - 1)));
            }
            let body = serde_json::json!({"data":[{"id":format!("p{page}")}],
                "has_more":true,"last_id":format!("p{page}")})
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\
                Connection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let r = w.app.providers().test_provider(input(url)).await.unwrap();
    assert!(r.ok);
    assert_eq!(r.models.len(), 5);
    server.await.unwrap();
}

#[tokio::test]
async fn oversized_body_without_content_length_is_unexpected() {
    let w = world().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = [0; 4096];
        assert!(socket.read(&mut bytes).await.unwrap() > 0);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let _ = socket.write_all(&vec![b'x'; 8 * 1024 * 1024 + 1]).await;
    });
    let r = w.app.providers().test_provider(input(url)).await.unwrap();
    assert_eq!(serde_json::to_value(r.reason).unwrap(), "unexpected");
    server.await.unwrap();
}

//! Images cross the real HTTP, command, storage and ACP boundaries.
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use base64::Engine;
use serde_json::{Value, json};
use shadows_core::testing::acp;
use shadows_core::{OperationId, ThreadId};
use tokio_stream::StreamExt;
use tower::ServiceExt;
#[path = "fixtures/app.rs"]
mod app;
use app::{App, call, default_settings, get_json, post, test_app, wait_terminal};

const STAGE: &str = concat!(
    "[Shadows] Stage: idea. Nothing is written yet: discuss the idea; ",
    "when the person asks how to start, walk the vision fields one at a time."
);
const RED: &[u8] = include_bytes!("fixtures/red.png");
const BLUE: &[u8] = include_bytes!("fixtures/blue.png");

async fn upload(app: &App, thread: &str, command: &str, media: &str, bytes: &[u8]) -> (u16, Value) {
    let boundary = "image-boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"command_id\"\r\n\r\n{command}\r\n\
         --{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"image.png\"\r\n\
         Content-Type: {media}\r\n\r\n"
    )
    .into_bytes();
    body.extend(bytes);
    body.extend(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = app
        .router
        .clone()
        .oneshot(
            Request::post(format!("/api/threads/{thread}/attachments"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn image(app: &App, command: &str, bytes: &[u8]) -> String {
    let (status, record) = upload(app, app.thread.as_str(), command, "image/png", bytes).await;
    assert_eq!(status, 200, "{record}");
    record["id"].as_str().unwrap().into()
}

fn body(command: &str, text: &str, ids: &[&str]) -> Value {
    let mut b = default_settings();
    b["command_id"] = json!(command);
    b["prompt"] = json!(text);
    b["attachments"] = json!(ids);
    b
}

async fn send(app: &App, b: Value) -> (u16, Value) {
    post(app, &format!("/api/threads/{}/turns", app.thread), b).await
}

async fn finish(app: &App, command: &str, text: &str, ids: &[&str]) -> Value {
    let (status, answer) = send(app, body(command, text, ids)).await;
    assert_eq!(status, 202, "{answer}");
    let op = OperationId::from_literal(answer["operation_id"].as_str().unwrap());
    assert_eq!(wait_terminal(app, &op).await.status_kind, "Completed");
    answer
}

async fn captured(app: &App, name: &str) -> Value {
    let context = shadows_core::Threads::turn_context_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    let path = context.project_directory.unwrap().join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn block(bytes: &[u8]) -> Value {
    json!({"type":"image", "mimeType":"image/png",
        "data":base64::engine::general_purpose::STANDARD.encode(bytes)})
}

async fn counts(app: &App) -> (i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM command_record),
        (SELECT COUNT(*) FROM durable_event), (SELECT COUNT(*) FROM operation),
        (SELECT COUNT(*) FROM thread_entry)",
    )
    .fetch_one(app.storage.reader())
    .await
    .unwrap()
}

#[tokio::test]
async fn upload_validates_content_type_size_and_replay() {
    let app = test_app().await;
    let before = counts(&app).await;
    for (command, media, bytes) in [
        ("bad", "image/png", b"not an image".as_slice()),
        ("truncated", "image/png", &RED[..24]),
        ("mismatch", "image/jpeg", RED),
        ("large", "image/png", &vec![0; 5_242_881]),
    ] {
        let (status, answer) = upload(&app, app.thread.as_str(), command, media, bytes).await;
        assert_eq!(status, 422, "{answer}");
        if command == "large" {
            assert!(answer["message"].as_str().unwrap().contains("5 MiB"));
        }
        assert_eq!(counts(&app).await, before);
    }
    let id = image(&app, "upload", RED).await;
    let (_, replay) = upload(&app, app.thread.as_str(), "upload", "image/png", RED).await;
    assert_eq!(replay["id"], id);
    let (status, conflict) = upload(&app, app.thread.as_str(), "upload", "image/png", BLUE).await;
    assert_eq!(
        (status, conflict["code"].as_str()),
        (409, Some("COMMAND_CONFLICT"))
    );
    let response = app
        .router
        .clone()
        .oneshot(
            Request::get(format!("/api/threads/{}/attachments/{id}", app.thread))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["content-type"], "image/png");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .as_ref(),
        RED
    );
    let digest: Vec<u8> = sqlx::query_scalar("SELECT sha256 FROM attachment WHERE id=?")
        .bind(&id)
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    let digest = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(
        digest,
        "b1ff9c8ea3a780bad09b346c423d2d0e46815926879b18e841d928376a946640"
    );
}

#[tokio::test]
async fn text_and_two_images_reach_acp_in_exact_order() {
    let app = test_app().await;
    let red = image(&app, "red", RED).await;
    let blue = image(&app, "blue", BLUE).await;
    let mut subscription = app::subscribe(&app, &app.thread).await;
    finish(&app, "send", "hello", &[&red, &blue]).await;
    let blocks = captured(&app, "image-prompt.json").await;
    assert_eq!(
        blocks,
        json!([
            {"type":"text", "text":"hello"}, block(RED), block(BLUE),
            {"type":"text", "text":STAGE}
        ])
    );
    let entries: Vec<Value> = get_json(&app, &format!("/api/threads/{}/entries", app.thread)).await;
    assert_eq!(
        entries[0]["attachments"],
        json!([
            {"id":red,"media_type":"image/png","name":"image.png"},
            {"id":blue,"media_type":"image/png","name":"image.png"}
        ])
    );
    assert!(entries[0].get("mimeType").is_none());
    assert!(entries[0]["attachments"][0].get("data").is_none());
    let stored: String = sqlx::query_scalar("SELECT attachments_json FROM thread_entry WHERE id=?")
        .bind(entries[0]["id"].as_str().unwrap())
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&stored).unwrap(),
        entries[0]["attachments"]
    );
    let doc: Value = serde_json::from_str(&shadows_http::openapi_document()).unwrap();
    let props = doc["components"]["schemas"]["AttachmentRef"]["properties"]
        .as_object()
        .unwrap();
    assert_eq!(
        props.keys().map(String::as_str).collect::<Vec<_>>(),
        ["id", "media_type", "name"]
    );
    let frames = app::frames_until(&mut subscription, |event, data| {
        event == "durable"
            && data["kind"] == "ThreadEntryAppended"
            && data["payload"]["kind"] == "UserMessage"
    })
    .await;
    let payload = &frames.last().unwrap().1["payload"];
    assert_eq!(payload["attachments"], entries[0]["attachments"]);
    let before = counts(&app).await;
    let (status, conflict) = send(&app, body("send", "hello", &[&blue, &red])).await;
    assert_eq!(
        (status, conflict["code"].as_str()),
        (409, Some("COMMAND_CONFLICT"))
    );
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn image_only_turn_and_attachment_replay_conflict() {
    let app = test_app().await;
    let red = image(&app, "red", RED).await;
    let blue = image(&app, "blue", BLUE).await;
    let first = finish(&app, "send", "", &[&red]).await;
    assert_eq!(
        captured(&app, "image-prompt.json").await,
        json!([{"type":"text","text":""}, block(RED), {"type":"text","text":STAGE}])
    );
    let (status, replay) = send(&app, body("send", "", &[&red])).await;
    assert_eq!(status, 202);
    assert_eq!(first, replay);
    let before = counts(&app).await;
    let (status, conflict) = send(&app, body("send", "", &[&blue])).await;
    assert_eq!(
        (status, conflict["code"].as_str()),
        (409, Some("COMMAND_CONFLICT"))
    );
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn foreign_or_unknown_images_are_refused_before_writes() {
    let app = test_app().await;
    let other = app::create_thread(&app, json!({"command_id":"other","title":"Other"})).await;
    let (_, foreign) = upload(
        &app,
        other["id"].as_str().unwrap(),
        "foreign",
        "image/png",
        RED,
    )
    .await;
    let unknown = uuid::Uuid::new_v4().to_string();
    let before = counts(&app).await;
    for id in [foreign["id"].as_str().unwrap(), unknown.as_str()] {
        let response = app
            .router
            .clone()
            .oneshot(
                Request::get(format!("/api/threads/{}/attachments/{id}", app.thread))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 404);
        let (status, _) = send(&app, body("refused", "hello", &[id])).await;
        assert_eq!(status, 404);
        let (status, _) = post(
            &app,
            &format!("/api/threads/{}/queue", app.thread),
            body("refused-queue", "hello", &[id]),
        )
        .await;
        assert_eq!(status, 404);
        assert_eq!(counts(&app).await, before);
    }
    assert_eq!(app.sessions.live_count().await, 0);
}

#[tokio::test]
async fn fork_keeps_references_and_removal_reclaims_unreferenced_bytes() {
    let app = test_app().await;
    let red = image(&app, "red", RED).await;
    image(&app, "unused", BLUE).await;
    finish(&app, "send", "hello", &[&red]).await;
    let entries: Vec<Value> = get_json(&app, &format!("/api/threads/{}/entries", app.thread)).await;
    let (status, fork) = post(
        &app,
        &format!("/api/threads/{}/fork", app.thread),
        json!({"command_id":"fork","at_entry_id":entries.last().unwrap()["id"]}),
    )
    .await;
    assert_eq!(status, 201, "{fork}");
    let fork_id = fork["id"].as_str().unwrap();
    let copied: Vec<Value> = get_json(&app, &format!("/api/threads/{fork_id}/entries")).await;
    assert_eq!(copied[0]["attachments"], entries[0]["attachments"]);
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/threads/{}?command_id=remove", app.thread),
            None
        )
        .await
        .0,
        200
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/threads/{}/attachments/{red}", app.thread),
            None
        )
        .await
        .0,
        404
    );
    let bytes = app
        .core
        .threads()
        .attachment(
            &ThreadId::from_literal(fork_id),
            &shadows_core::AttachmentId::from_literal(&red),
        )
        .await
        .unwrap()
        .1;
    assert_eq!(bytes, RED);
    let retained: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM attachment")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(
        retained, 1,
        "the fork retains only images referenced by its copied messages"
    );
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/threads/{fork_id}?command_id=remove-fork"),
            None
        )
        .await
        .0,
        200
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM attachment")
        .fetch_one(app.storage.reader())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn queued_and_steered_messages_keep_images() {
    let app = test_app().await;
    let red = image(&app, "red", RED).await;
    let blue = image(&app, "blue", BLUE).await;
    let op = app::start_settled(&app, "steerable").await;
    let path = format!("/api/threads/{}/queue", app.thread);
    let queue_body = body("queue", "left", &[&blue, &red]);
    let (status, queued) = post(&app, &path, queue_body.clone()).await;
    assert_eq!(status, 202, "{queued}");
    assert_eq!(queued["message"]["attachments"], json!([blue, red]));
    let before = counts(&app).await;
    let (status, conflict) = post(&app, &path, body("queue", "left", &[&red, &blue])).await;
    assert_eq!(
        (status, conflict["code"].as_str()),
        (409, Some("COMMAND_CONFLICT"))
    );
    assert_eq!(counts(&app).await, before);
    assert_eq!(post(&app, &path, queue_body).await.1, queued);
    let id = queued["message"]["id"].as_str().unwrap();
    let command_count = counts(&app).await.0;
    let (status, refused) = post(
        &app,
        &format!("{path}/{id}/send-now"),
        json!({"command_id":"mismatched-steer", "attachments":[red,blue]}),
    )
    .await;
    assert_eq!(status, 422, "{refused}");
    assert_eq!(refused["code"], "INVALID_COMMAND");
    assert_eq!(counts(&app).await.0, command_count);
    let (status, answer) = post(
        &app,
        &format!("{path}/{id}/send-now"),
        json!({"command_id":"steer", "attachments":[blue,red]}),
    )
    .await;
    assert_eq!(status, 202, "{answer}");
    assert_eq!(answer["status"], "steered");
    let before = counts(&app).await;
    let (status, replay) = post(
        &app,
        &format!("{path}/{id}/send-now"),
        json!({"command_id":"steer", "attachments":[blue,red]}),
    )
    .await;
    assert_eq!(status, 202, "{replay}");
    assert_eq!(replay, answer);
    let (status, conflict) = post(
        &app,
        &format!("{path}/{id}/send-now"),
        json!({"command_id":"steer", "attachments":[red,blue]}),
    )
    .await;
    assert_eq!(
        (status, conflict["code"].as_str()),
        (409, Some("COMMAND_CONFLICT"))
    );
    // Only the turn watcher may finish while these replay requests are served.
    let after = counts(&app).await;
    assert_eq!(after.0, before.0);
    wait_terminal(&app, &op).await;
    assert_eq!(
        captured(&app, "image-steer.json").await,
        json!([
            {"type":"text","text":format!("left\n\n{STAGE}")}, block(BLUE), block(RED)
        ])
    );
    let entries = app::entries(&app).await;
    let steered = entries.iter().find(|e| e.body == "left").unwrap();
    assert_eq!(steered.attachments[0].id.as_str(), blue);
    assert_eq!(steered.attachments[1].id.as_str(), red);
    // A waiting image is also sent by the completion worker, without Send now.
    let op = app::start_settled(&app, "wait-for-release").await;
    let (status, _) = post(&app, &path, body("automatic", "", &[&red])).await;
    assert_eq!(status, 202);
    let context = shadows_core::Threads::turn_context_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    std::fs::write(context.project_directory.unwrap().join("release"), "").unwrap();
    wait_terminal(&app, &op).await;
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let ops = app.core.turns().operations(&app.thread).await.unwrap();
            if ops.len() == 3 && ops[0].finished_at.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        captured(&app, "image-prompt.json").await,
        json!([
            {"type":"text","text":""}, block(RED), {"type":"text","text":STAGE}
        ])
    );
}

#[tokio::test]
async fn a_harness_without_image_capability_refuses_with_its_reason() {
    let app = test_app().await;
    let context = shadows_core::Threads::turn_context_for_test(&app.storage, &app.thread)
        .await
        .unwrap();
    std::fs::write(
        context
            .project_directory
            .unwrap()
            .join(".fake-acp-no-images"),
        "",
    )
    .unwrap();
    let red = image(&app, "red", RED).await;
    let cold_before = counts(&app).await;
    let mut observer = shadows_core::testing::DatabaseWrites::observe(&app.storage)
        .await
        .unwrap();
    let version = observer.version().await.unwrap();
    let (status, refused) = send(&app, body("cold-send", "", &[&red])).await;
    assert_eq!(status, 422, "{refused}");
    assert_eq!(
        refused["message"],
        "The harness does not support image prompts"
    );
    assert_eq!(counts(&app).await, cold_before);
    assert_eq!(observer.version().await.unwrap(), version);
    assert_eq!(app.sessions.live_count().await, 0);
    let (_, session) = post(
        &app,
        &format!("/api/threads/{}/session", app.thread),
        json!({}),
    )
    .await;
    assert_eq!(session["capabilities"]["image"], false);
    let reason = session["capabilities"]["image_reason"].as_str().unwrap();
    assert_eq!(reason, "The harness does not support image prompts");
    let before = counts(&app).await;
    let (status, refused) = send(&app, body("send", "", &[&red])).await;
    assert_eq!(status, 422, "{refused}");
    assert_eq!(refused["message"], reason);
    assert_eq!(counts(&app).await, before);
    let (status, refused) = post(
        &app,
        &format!("/api/threads/{}/queue", app.thread),
        body("queue", "", &[&red]),
    )
    .await;
    assert_eq!(status, 422, "{refused}");
    assert_eq!(refused["message"], reason);
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn accepted_types_are_decoded_by_content_and_the_size_boundary_is_inclusive() {
    let app = test_app().await;
    for (media, bytes) in [
        ("image/png", RED),
        ("image/jpeg", include_bytes!("fixtures/red.jpeg").as_slice()),
        ("image/gif", include_bytes!("fixtures/red.gif").as_slice()),
        ("image/webp", include_bytes!("fixtures/red.webp").as_slice()),
    ] {
        let (status, answer) = upload(&app, app.thread.as_str(), media, media, bytes).await;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["media_type"], media);
        assert_eq!(answer["byte_len"], bytes.len());
    }
    let mut boundary = RED.to_vec();
    boundary.resize(5_242_880, 0);
    let (status, answer) = upload(
        &app,
        app.thread.as_str(),
        "boundary",
        "image/png",
        &boundary,
    )
    .await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["byte_len"], 5_242_880);
}

#[tokio::test]
async fn a_truncated_later_gif_frame_is_refused_before_writes() {
    let app = test_app().await;
    let gif = include_bytes!("fixtures/red.gif");
    let mut animation = gif[..gif.len() - 1].to_vec();
    animation.extend(&gif[25..]);
    let (status, answer) =
        upload(&app, app.thread.as_str(), "valid", "image/gif", &animation).await;
    assert_eq!(status, 200, "{answer}");
    let before = counts(&app).await;
    let mut truncated = gif[..gif.len() - 1].to_vec();
    truncated.extend([0x2c, 0]);
    let (status, answer) = upload(
        &app,
        app.thread.as_str(),
        "truncated-frame",
        "image/gif",
        &truncated,
    )
    .await;
    assert_eq!(status, 422, "{answer}");
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn a_truncated_later_png_frame_is_refused_before_writes() {
    let app = test_app().await;
    let png = include_bytes!("fixtures/animated.png");
    let (status, answer) = upload(&app, app.thread.as_str(), "valid", "image/png", png).await;
    assert_eq!(status, 200, "{answer}");
    let before = counts(&app).await;
    let frame = png.windows(4).position(|b| b == b"fdAT").unwrap();
    let truncated = &png[..frame + 10];
    let (status, answer) = upload(
        &app,
        app.thread.as_str(),
        "truncated-frame",
        "image/png",
        truncated,
    )
    .await;
    assert_eq!(status, 422, "{answer}");
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn a_truncated_later_webp_frame_is_refused_before_writes() {
    let app = test_app().await;
    let webp = include_bytes!("fixtures/animated.webp");
    let (status, answer) = upload(&app, app.thread.as_str(), "valid", "image/webp", webp).await;
    assert_eq!(status, 200, "{answer}");
    let before = counts(&app).await;
    let frame = webp.windows(4).rposition(|b| b == b"ANMF").unwrap();
    let mut truncated = webp[..frame + 26].to_vec();
    let size = (truncated.len() as u32 - 8).to_le_bytes();
    truncated[4..8].copy_from_slice(&size);
    let (status, answer) = upload(
        &app,
        app.thread.as_str(),
        "truncated-frame",
        "image/webp",
        &truncated,
    )
    .await;
    assert_eq!(status, 422, "{answer}");
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn a_damaged_later_webp_frame_is_refused_before_writes() {
    let app = test_app().await;
    let before = counts(&app).await;
    let mut damaged = include_bytes!("fixtures/animated.webp").to_vec();
    let frame = damaged.windows(4).rposition(|b| b == b"VP8L").unwrap();
    damaged[frame + 13..].fill(0xff);
    let (status, answer) = upload(
        &app,
        app.thread.as_str(),
        "damaged-frame",
        "image/webp",
        &damaged,
    )
    .await;
    assert_eq!(status, 422, "{answer}");
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn oversized_decoded_images_are_refused_before_writes() {
    let app = test_app().await;
    let before = counts(&app).await;
    for (command, width, height) in [("wide", 8193_u16, 1_u16), ("allocation", 4097, 4097)] {
        let mut gif = include_bytes!("fixtures/red.gif").to_vec();
        gif[6..8].copy_from_slice(&width.to_le_bytes());
        gif[8..10].copy_from_slice(&height.to_le_bytes());
        let (status, answer) = upload(&app, app.thread.as_str(), command, "image/gif", &gif).await;
        assert_eq!(status, 422, "{answer}");
        assert_eq!(counts(&app).await, before);
    }
    let mut webp = include_bytes!("fixtures/animated.webp").to_vec();
    webp[24..27].copy_from_slice(&4095_u32.to_le_bytes()[..3]);
    webp[27..30].copy_from_slice(&4095_u32.to_le_bytes()[..3]);
    let (status, answer) = upload(
        &app,
        app.thread.as_str(),
        "animated-allocation",
        "image/webp",
        &webp,
    )
    .await;
    assert_eq!(status, 422, "{answer}");
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn an_animation_with_excessive_total_decoded_bytes_is_refused() {
    let app = test_app().await;
    let gif = include_bytes!("fixtures/red.gif");
    let mut animation = gif[..25].to_vec();
    animation[6..8].copy_from_slice(&256_u16.to_le_bytes());
    animation[8..10].copy_from_slice(&256_u16.to_le_bytes());
    for _ in 0..257 {
        animation.extend(&gif[25..gif.len() - 1]);
    }
    animation.push(0x3b);
    let before = counts(&app).await;
    let (status, answer) = upload(
        &app,
        app.thread.as_str(),
        "animation-bomb",
        "image/gif",
        &animation,
    )
    .await;
    assert_eq!(status, 422, "{answer}");
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn the_upload_body_limit_stops_an_unbounded_stream() {
    let app = test_app().await;
    let before = counts(&app).await;
    let polled = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = polled.clone();
    let prefix = b"--b\r\nContent-Disposition: form-data; name=\"file\"; \
        filename=\"image.png\"\r\nContent-Type: image/png\r\n\r\n"
        .to_vec();
    let stream =
        tokio_stream::iter(std::iter::once(prefix).chain(std::iter::repeat_with(|| vec![0; 4096])))
            .map(move |chunk| {
                observed.fetch_add(chunk.len(), std::sync::atomic::Ordering::SeqCst);
                Ok::<_, std::io::Error>(chunk)
            });
    let response = app.router.clone().oneshot(
        Request::post(format!("/api/threads/{}/attachments", app.thread))
            .header("content-type", "multipart/form-data; boundary=b")
            .body(Body::from_stream(stream))
            .unwrap(),
    );
    let response = tokio::time::timeout(std::time::Duration::from_secs(10), response)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), 422);
    assert!(
        polled.load(std::sync::atomic::Ordering::SeqCst)
            <= shadows_core::MAX_IMAGE_BYTES + 65536 + 4096
    );
    assert_eq!(counts(&app).await, before);
}

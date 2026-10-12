//! Non-streaming GET routes do not commit database writes.
use super::*;
use serde_json::json;
#[path = "../fixtures/app.rs"]
mod app;
#[path = "../fixtures/plan.rs"]
mod plan;

/// Covers successful default reads plus required code query arguments.
/// SSE and optional-query combinations are outside this test's scope.
#[tokio::test]
async fn no_get_route_changes_state() {
    use shadows_core::{DesignOp, OutcomeContent, OutcomeId, PartContent, PartId, VisionField};
    let a = app::test_app().await;
    a.storage.seed_map_for_tests(&a.project).await.unwrap();
    let part = PartId::generate();
    let outcome = OutcomeId::generate();
    let mut ops: Vec<_> = [
        VisionField::Purpose,
        VisionField::Users,
        VisionField::Goals,
        VisionField::Boundaries,
        VisionField::TechnicalDirection,
    ]
    .into_iter()
    .map(|field| DesignOp::VisionItemAdd {
        field,
        text: "Complete".into(),
    })
    .collect();
    ops.push(DesignOp::PartCreate {
        id: part.clone(),
        parent: None,
        before: None,
        content: PartContent {
            title: "Part".into(),
            responsibility: "Owns logic".into(),
            ..Default::default()
        },
    });
    ops.push(DesignOp::OutcomeCreate {
        id: outcome.clone(),
        parent: None,
        before: None,
        content: OutcomeContent {
            title: "Ship".into(),
            intended_result: "Delivered".into(),
            acceptance: vec!["Works".into()],
        },
    });
    a.core
        .design()
        .edit("seed-design".into(), &a.project, 0, ops)
        .await
        .unwrap();
    let workflow = plan::approved_v1(&a).await;
    let version = a.core.plans().get(&workflow).await.unwrap();
    let path = format!("/api/projects/{}/agreements", a.project);
    let (status, agreement) = app::post(
        &a,
        &path,
        json!({"command_id":"agreement",
        "content":{"capability":"Read","purpose":"Read","behavior":"",
        "acceptance":[],"parties":[],"openapi":{}}}),
    )
    .await;
    assert_eq!(status, 200, "{agreement}");
    let op = app::start_settled(&a, "hang").await;
    for _ in 0..200 {
        if a.handles.running_for(&a.thread).await.as_ref() == Some(&op) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    let (grant, _) = a.storage.issue_thread_grant(&a.thread).await.unwrap();
    a.core
        .design()
        .propose_vision(
            &grant,
            serde_json::from_value(json!({"action":"add",
        "field":"goals","text":"Future"}))
            .unwrap(),
        )
        .await
        .unwrap();
    a.core
        .design()
        .propose_map(
            &grant,
            serde_json::from_value(json!({"action":"add",
        "parts":[{"key":"future","content":{"title":"Future","responsibility":"Future",
            "design":"","serves":["G1"],"folders":["future"]}}]}))
            .unwrap(),
        )
        .await
        .unwrap();
    a.core.turns().stop(&op).await.unwrap();
    let mut observer = shadows_core::testing::database_writes(&a.storage)
        .await
        .unwrap();
    // Prove the detector sees UPDATEs with unchanged row counts.
    let before = observer.version().await.unwrap();
    sqlx::query("UPDATE project SET name='Updated' WHERE id=?")
        .bind(a.project.as_str())
        .execute(a.storage.reader())
        .await
        .unwrap();
    assert_ne!(before, observer.version().await.unwrap());
    for (template, methods) in document()["paths"].as_object().unwrap() {
        let Some(get) = methods.get("get") else {
            continue;
        };
        if get["responses"]["200"]["content"]
            .get("text/event-stream")
            .is_some()
        {
            continue;
        }
        let mut path = template.clone();
        for parameter in get["parameters"].as_array().into_iter().flatten() {
            if parameter["in"] != "path" {
                continue;
            }
            let name = parameter["name"].as_str().unwrap();
            let value = match name {
                "id" if template.starts_with("/api/projects/") => a.project.to_string(),
                "id" if template.starts_with("/api/threads/") => a.thread.to_string(),
                "id" if template.starts_with("/api/workflows/") => workflow.to_string(),
                "id" if template.starts_with("/api/plans/") => version.plan_id.to_string(),
                "agreement" => agreement["agreement_id"].as_str().unwrap().to_string(),
                "part" => part.to_string(),
                "outcome" => outcome.to_string(),
                _ => panic!("unseeded path parameter {name} in {template}"),
            };
            path = path.replace(&format!("{{{name}}}"), &value);
        }
        if template.ends_with("/code/definitions") || template.ends_with("/code/references") {
            path.push_str("?name=unknown");
        } else if template.ends_with("/code/outline") {
            path.push_str("?path=");
        }
        let before = observer.version().await.unwrap();
        let (status, body) = app::call(&a, "GET", &path, None).await;
        assert!((200..300).contains(&status), "{path}: {status} {body}");
        assert_eq!(
            observer.version().await.unwrap(),
            before,
            "GET wrote: {path}"
        );
    }
    app::shut_down_app(a).await;
}

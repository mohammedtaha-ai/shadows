//! The project map captures latest plans with aggregated cross-plan edges.

use serde_json::{Value, json};
use shadows_core::testing::acp;
use shadows_core::{Link, LinkKind, PlanOp, TaskParent};

#[path = "fixtures/app.rs"]
mod app;
#[path = "fixtures/plan.rs"]
mod plan;

use app::{get_json, other_project, test_app};
use plan::{add, draft, draft_on, edit};

#[tokio::test]
async fn the_project_map_aggregates_task_links_with_foreign_plan_metadata() {
    let app = test_app().await;
    let (foreign, thread) = other_project(&app).await;
    let target = draft_on(&app, &thread, "target").await;
    edit(&app, &target.workflow_id, 0, &[add(3)]).await;
    app.core
        .code()
        .link("link".into(), &app.project, &foreign)
        .await
        .unwrap();
    let source = draft(&app).await;
    let dependency = |task| PlanOp::LinkPut {
        link: Link {
            task,
            after: TaskParent::Plan {
                plan_id: target.plan_id.clone(),
                task: 3,
            },
            kind: LinkKind::Needs,
            label: "Login API".into(),
            waiting_items: vec![],
        },
    };
    edit(
        &app,
        &source.workflow_id,
        0,
        &[add(4), add(5), dependency(4), dependency(5)],
    )
    .await;
    let path = format!("/api/projects/{}/plan-map", app.project);
    let map: Value = get_json(&app, &path).await;
    assert_eq!(map["plans"].as_array().unwrap().len(), 2);
    let theirs = map["plans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["plan_id"] == json!(target.plan_id))
        .unwrap();
    assert_eq!(theirs["project_id"], json!(foreign));
    assert_eq!(theirs["project_name"], "Other");
    assert_eq!(theirs["task_count"], 1);
    assert_eq!(theirs["workflow_id"], json!(target.workflow_id));
    assert_eq!(
        map["links"],
        json!([{
            "plan_id": source.plan_id, "after": target.plan_id, "count": 2, "broken": false
        }])
    );
    app.core
        .code()
        .unlink("unlink".into(), &app.project, &foreign)
        .await
        .unwrap();
    let broken: Value = get_json(&app, &path).await;
    assert_eq!(broken["links"][0]["broken"], true);
    assert_eq!(broken["plans"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn the_map_keeps_incoming_foreign_links_to_archived_plans() {
    let app = test_app().await;
    let target = draft(&app).await;
    edit(&app, &target.workflow_id, 0, &[add(4)]).await;
    let (foreign, thread) = other_project(&app).await;
    app.core
        .code()
        .link("link".into(), &foreign, &app.project)
        .await
        .unwrap();
    let source = draft_on(&app, &thread, "source").await;
    edit(
        &app,
        &source.workflow_id,
        0,
        &[
            add(3),
            PlanOp::LinkPut {
                link: Link {
                    task: 3,
                    after: TaskParent::Plan {
                        plan_id: target.plan_id.clone(),
                        task: 4,
                    },
                    kind: LinkKind::Needs,
                    label: "Screen".into(),
                    waiting_items: vec![],
                },
            },
        ],
    )
    .await;
    let incoming: Value = get_json(&app, &format!("/api/projects/{}/plan-map", app.project)).await;
    assert_eq!(incoming["plans"].as_array().unwrap().len(), 2);
    assert_eq!(incoming["links"][0]["plan_id"], json!(source.plan_id));
    assert_eq!(incoming["links"][0]["after"], json!(target.plan_id));
    app.core
        .plans()
        .archive("archive".into(), &target.plan_id)
        .await
        .unwrap();
    let map: Value = get_json(&app, &format!("/api/projects/{foreign}/plan-map")).await;
    let archived = map["plans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["plan_id"] == json!(target.plan_id))
        .unwrap();
    assert_eq!(archived["plan_state"], "Archived");
    assert_eq!(map["links"][0]["broken"], false);
    let own: Value = get_json(&app, &format!("/api/projects/{}/plan-map", app.project)).await;
    assert!(own["plans"].as_array().unwrap().is_empty());
}

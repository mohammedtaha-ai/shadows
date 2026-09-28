//! Spec §14.4: the application is one `AppCore`; adapters and shutdown reach it through that.

#[path = "fixtures/acp.rs"]
mod acp;
#[path = "fixtures/app.rs"]
mod app;

use app::{call, default_settings, start_on, test_app, wait_terminal};

#[tokio::test]
async fn the_router_is_built_from_the_core() {
    let app = test_app().await;
    let (status, list) = call(&app, "GET", "/api/projects", None).await;
    assert_eq!(status, 200);
    // `app::names` reads ids; a project's id is generated, so its name is read.
    assert_eq!(list[0]["name"], "Demo");
    assert_eq!(list[0]["id"], app.project.as_str());
    assert_eq!(list.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn shut_down_through_the_core_cancels_and_records() {
    let app = test_app().await;
    let op = start_on(&app, app.thread.as_str(), "hang", default_settings()).await;
    let kind = app
        .core
        .shut_down(std::time::Duration::from_secs(10), std::future::pending())
        .await
        .unwrap();
    assert_eq!(kind, shadows_core::StopKind::Graceful);
    assert_eq!(wait_terminal(&app, &op).await.status_kind, "Cancelled");
}

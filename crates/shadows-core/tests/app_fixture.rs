//! The restricted fixture: `testing::app` starts `AppCore` over a database
//! path, and a test reaches services through its accessors only.
use std::time::Duration;

use shadows_core::{CoreError, testing};

#[tokio::test]
async fn the_restricted_fixture_starts_and_serves_through_its_accessors() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("core.db");
    let directory = tmp.path().join("project");
    std::fs::create_dir_all(&directory).unwrap();

    let app = testing::app(db.clone()).await.unwrap();
    let created = app
        .project_lifecycle()
        .create(
            "create".into(),
            "fixture",
            "Fixture",
            directory.to_str().unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        app.harness().aylul_profile(),
        Err(CoreError::AylulNotConfigured)
    ));
    app.shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
    drop(app);

    // The same database at `db_path`: a second start reads what the first wrote.
    let again = testing::app(db).await.unwrap();
    let listed = again.projects().list().await.unwrap();
    assert_eq!(
        listed.iter().map(|p| &p.id).collect::<Vec<_>>(),
        vec![&created.id]
    );
    again
        .shut_down(Duration::from_secs(5), std::future::pending())
        .await
        .unwrap();
}

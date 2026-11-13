use core_db::Storage;
use core_projects::Projects;
use core_threads::{PlanningThread, Threads};
use std::sync::Arc;

struct Fixture {
    tmp: tempfile::TempDir,
    storage: Arc<Storage>,
    threads: Threads,
    visible: PlanningThread,
    hidden: PlanningThread,
}

async fn fixture() -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Storage::open(&tmp.path().join("test.db")).await.unwrap());
    let projects = Projects::new(storage.clone());
    let project = projects
        .ensure_assistant("aylul", "AYLUL", tmp.path())
        .await
        .unwrap();
    let threads = Threads::new(storage.clone(), projects);
    let project_id = project.id.clone();
    let hidden = storage
        .write_txn_as(move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO aylul_provider
            (slug, name, base_url, models, context_window, created_at, updated_at)
            VALUES ('p', 'Provider', 'https://example.com', '[]', 1000, '', '')",
                )
                .execute(&mut *conn)
                .await
                .unwrap();
                sqlx::query(
                    "INSERT INTO aylul_agent
            (slug, name, provider, default_model, created_at, updated_at)
            VALUES ('ay', 'Agent', 'p', 'model', '', '')",
                )
                .execute(&mut *conn)
                .await
                .unwrap();
                Threads::create_hidden_in(conn, &project_id, "Group", "ay", &Storage::now()).await
            })
        })
        .await
        .unwrap();
    let visible = threads
        .create(
            "visible".into(),
            &project.id,
            "Direct",
            Some("aylul"),
            Some("ay"),
        )
        .await
        .unwrap();
    Fixture {
        tmp,
        storage,
        threads,
        visible,
        hidden,
    }
}

#[tokio::test]
async fn a_hidden_thread_is_left_out_of_the_agent_list() {
    let f = fixture().await;
    let profile = f.threads.list_for_profile("ay").await.unwrap();
    let project = f.threads.list(&f.visible.project_id).await.unwrap();
    for threads in [profile, project] {
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, f.visible.id);
    }
    assert_eq!(f.threads.get(&f.hidden.id).await.unwrap().id, f.hidden.id);
}

#[tokio::test]
async fn a_hidden_thread_is_still_closed_with_its_agent() {
    let f = fixture().await;
    let mut conn = f.storage.reader().acquire().await.unwrap();
    let threads = Threads::list_for_profile_in(&mut conn, "ay").await.unwrap();
    assert_eq!(threads.len(), 2);
    assert!(threads.iter().any(|thread| thread.id == f.hidden.id));
}

#[tokio::test]
async fn a_hidden_thread_requires_the_assistant_project() {
    let f = fixture().await;
    let dir = f.tmp.path().join("ordinary");
    std::fs::create_dir(&dir).unwrap();
    let project = Projects::new(f.storage.clone())
        .create(
            "ordinary".into(),
            "ordinary",
            "Ordinary",
            dir.to_str().unwrap(),
        )
        .await
        .unwrap();
    let result = f
        .storage
        .write_txn_as(move |conn| {
            Box::pin(async move {
                Threads::create_hidden_in(conn, &project.id, "Group", "ay", &Storage::now()).await
            })
        })
        .await;
    assert!(matches!(
        result,
        Err(core_threads::ThreadsError::Projects(
            core_projects::ProjectsError::SettingNotOffered { .. }
        )),
    ));
}

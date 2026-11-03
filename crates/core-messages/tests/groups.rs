mod support;

use core_threads::Threads;
use support::fixture;

#[tokio::test]
async fn a_group_gets_one_hidden_thread_per_member() {
    let f = fixture().await;
    let slugs = ["alpha".into(), "beta".into()];
    let group = f
        .messages
        .create_group("group".into(), "Group", &slugs)
        .await
        .unwrap();
    let replay = f
        .messages
        .create_group("group".into(), "Group", &slugs)
        .await
        .unwrap();
    assert_eq!(group.id, replay.id);
    assert_eq!(group.members, replay.members);
    assert_eq!(group.members.len(), 2);
    for member in group.members {
        assert!(
            f.threads
                .list_for_profile(&member.slug)
                .await
                .unwrap()
                .is_empty()
        );
        let thread = f.threads.get(&member.thread_id).await.unwrap();
        assert_eq!(thread.profile.as_deref(), Some(member.slug.as_str()));
    }
}

#[tokio::test]
async fn a_group_refuses_seven_members() {
    let f = fixture().await;
    let slugs = (0..7).map(|i| format!("agent-{i}")).collect::<Vec<_>>();
    assert!(
        f.messages
            .create_group("group".into(), "Group", &slugs)
            .await
            .is_err()
    );
    assert!(f.messages.groups().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_failed_group_creation_leaves_no_thread() {
    let f = fixture().await;
    let slugs = ["alpha".into(), "beta".into(), "missing".into()];
    assert!(
        f.messages
            .create_group("group".into(), "Group", &slugs)
            .await
            .is_err()
    );
    assert!(f.messages.groups().await.unwrap().is_empty());
    let mut conn = f.storage.reader().acquire().await.unwrap();
    assert!(
        Threads::list_for_profile_in(&mut conn, "alpha")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        Threads::list_for_profile_in(&mut conn, "beta")
            .await
            .unwrap()
            .is_empty()
    );
    let members: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message_group_member")
        .fetch_one(f.storage.reader())
        .await
        .unwrap();
    assert_eq!(members, 0);
}

#[tokio::test]
async fn a_group_refuses_a_deleted_agent() {
    let f = fixture().await;
    let agents = f.agents.clone();
    f.storage
        .write_txn_as(move |conn| {
            Box::pin(async move { agents.delete_in(conn, "delete".into(), "beta").await })
        })
        .await
        .unwrap();
    let slugs = ["alpha".into(), "beta".into()];
    assert!(
        f.messages
            .create_group("group".into(), "Group", &slugs)
            .await
            .is_err()
    );
    assert!(f.messages.groups().await.unwrap().is_empty());
    let mut conn = f.storage.reader().acquire().await.unwrap();
    assert!(
        Threads::list_for_profile_in(&mut conn, "alpha")
            .await
            .unwrap()
            .is_empty()
    );
}

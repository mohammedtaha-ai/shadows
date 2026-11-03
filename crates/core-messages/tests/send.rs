mod support;

use support::fixture;

#[tokio::test]
async fn a_mention_reaches_only_that_member() {
    use core_messages::Address;
    let f = fixture().await;
    let group = f
        .messages
        .create_group("group".into(), "Group", &["alpha".into(), "beta".into()])
        .await
        .unwrap();
    let message = f
        .messages
        .send("send".into(), &group.id, "@ALPHA hello")
        .await
        .unwrap();
    assert_eq!(
        message.to,
        vec![Address::Agent {
            slug: "alpha".into()
        }]
    );
    let replay = f
        .messages
        .send("send".into(), &group.id, "@ALPHA hello")
        .await
        .unwrap();
    assert_eq!(message.id, replay.id);
    let scheduled: Vec<String> =
        sqlx::query_scalar("SELECT slug FROM message_delivery WHERE group_id = ? ORDER BY slug")
            .bind(group.id.as_str())
            .fetch_all(f.storage.reader())
            .await
            .unwrap();
    assert_eq!(scheduled, ["alpha"]);
    let beta = &group.members[1];
    assert!(f.threads.entries(&beta.thread_id).await.unwrap().is_empty());
}

#[tokio::test]
async fn messages_come_back_in_seq_order() {
    let f = fixture().await;
    let group = f
        .messages
        .create_group("group".into(), "Group", &["alpha".into()])
        .await
        .unwrap();
    let first = f
        .messages
        .send("first".into(), &group.id, "One")
        .await
        .unwrap();
    let second = f
        .messages
        .send("second".into(), &group.id, "Two")
        .await
        .unwrap();
    assert_eq!((first.seq, second.seq), (1, 2));
    let messages = f
        .messages
        .messages(&group.id, Some(first.seq))
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, second.id);
}

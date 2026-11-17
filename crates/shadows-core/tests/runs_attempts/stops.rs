//! Durable notices for interrupted attempts and the person's Stop.
use super::*;

async fn running(f: &Fixture) -> core_runs::AttemptId {
    let thread = f.executor_thread(3).await;
    let id = f.chain_attempt(&thread, 3, 1, "running", None, false).await;
    std::fs::create_dir_all(f.root.join("backend/app")).unwrap();
    std::fs::write(f.root.join("backend/app/a.php"), "unfinished\n").unwrap();
    id
}

async fn notice(f: &Fixture, planner: &ThreadId) -> shadows_core::QueuedMessage {
    for _ in 0..200 {
        if let Some(message) = f
            .core
            .turns()
            .queued(planner)
            .await
            .unwrap()
            .into_iter()
            .next()
        {
            return message;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the Planner received no notice");
}

#[tokio::test]
async fn a_restart_tells_the_planner_of_each_interrupted_attempt() {
    let f = fixture().await;
    let planner = super::checks::prepare_planner(&f).await;
    let id = running(&f).await;
    f.core.runs().recover().await.unwrap();
    let message = notice(&f, &planner).await;
    let card = &message.notice.as_ref().unwrap().card;
    assert_eq!(card["attempt_id"], id.as_str());
    assert_eq!(card["outcome"], "interrupted");
    assert_eq!(card["left_files"], serde_json::json!(["backend/app/a.php"]));
    assert!(message.prompt.contains("was interrupted"));
    f.finish().await;
}

#[tokio::test]
async fn a_second_restart_sends_no_second_interrupted_notice() {
    let f = fixture().await;
    let planner = super::checks::prepare_planner(&f).await;
    running(&f).await;
    f.core.runs().recover().await.unwrap();
    notice(&f, &planner).await;
    f.core.runs().recover().await.unwrap();
    assert_eq!(f.core.turns().queued(&planner).await.unwrap().len(), 1);
    f.finish().await;
}

#[tokio::test]
async fn the_persons_stop_tells_the_planner() {
    let f = fixture().await;
    let planner = super::checks::prepare_planner(&f).await;
    let id = running(&f).await;
    f.core.runs().stop(&id).await.unwrap();
    let message = notice(&f, &planner).await;
    let card = &message.notice.as_ref().unwrap().card;
    assert_eq!(card["outcome"], "stopped");
    assert!(card["reason"].is_null() && card["report"].is_null());
    assert!(message.prompt.contains("stopped by the person"));
    f.finish().await;
}

#[tokio::test]
async fn the_planners_stop_sends_no_notice() {
    let f = fixture().await;
    let planner = super::checks::prepare_planner(&f).await;
    let grant = planner_grant(&f).await;
    let id = running(&f).await;
    f.core
        .runs()
        .stop_for(&grant, "stop".into(), &id)
        .await
        .unwrap();
    assert!(f.core.turns().queued(&planner).await.unwrap().is_empty());
    f.finish().await;
}

#[tokio::test]
async fn a_persons_stop_notice_survives_a_failed_enqueue() {
    let f = fixture().await;
    let id = running(&f).await;
    f.core.runs().stop(&id).await.unwrap();
    let planner = super::checks::prepare_planner(&f).await;
    f.core.runs().recover().await.unwrap();
    let message = notice(&f, &planner).await;
    assert_eq!(message.notice.unwrap().card["outcome"], "stopped");
    f.finish().await;
}

#[tokio::test]
async fn an_owed_notice_is_sent_on_the_next_start() {
    let f = fixture().await;
    let id = running(&f).await;
    f.core.runs().recover().await.unwrap();
    // No Planner settings exist, so the first recovery cannot enqueue its notice.
    for _ in 0..100 {
        if f.core.runs().attempt(&id).await.unwrap().state == AttemptState::Interrupted {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let planner = super::checks::prepare_planner(&f).await;
    f.core.runs().recover().await.unwrap();
    let message = notice(&f, &planner).await;
    assert_eq!(message.notice.unwrap().card["outcome"], "interrupted");
    f.finish().await;
}

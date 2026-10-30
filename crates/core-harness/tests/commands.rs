mod fixture;

use core_threads::ThreadId;
use fixture::fixture;
use shadows_agent::events::SlashCommand;

fn one(name: &str) -> SlashCommand {
    SlashCommand {
        name: name.into(),
        description: String::new(),
        hint: None,
    }
}

#[tokio::test]
async fn a_new_list_replaces_the_old_and_forget_drops_it() {
    let (_tmp, storage, harness) = fixture().await;
    let thread = ThreadId::generate();
    let mut changed = harness.watch_commands();
    harness.report_commands_for_test(&thread, vec![one("a")]);
    harness.report_commands_for_test(&thread, vec![one("b")]);
    assert_eq!(harness.commands_of(&thread), Some(vec![one("b")]));
    assert_eq!(changed.try_recv().unwrap().1, vec![one("a")]);
    harness.close_session(&thread).await.unwrap();
    assert_eq!(harness.commands_of(&thread), None);

    storage.close().await.unwrap();
}

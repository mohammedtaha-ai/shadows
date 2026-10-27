use std::sync::Arc;

use core_agents::Agents;
use core_db::Storage;
use core_extensions::Extensions;
use core_grants::Grants;
use core_harness::{AylulOwners, Harness, SessionsConfig};
use core_instructions::Instructions;
use core_operations::Operations;
use core_person::PersonSettings;
use core_projects::Projects;
use core_providers::Providers;
use core_secrets::Secrets;
use core_threads::ThreadId;
use shadows_agent::{claude::ClaudeAdapter, events::SlashCommand};

async fn fixture() -> (tempfile::TempDir, Arc<Storage>, Arc<Harness>) {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(
        Storage::open(&tmp.path().join("harness.sqlite3"))
            .await
            .unwrap(),
    );
    let secrets = Secrets::new(storage.clone(), None);
    let providers = Providers::new(storage.clone(), secrets.clone());
    let projects = Projects::new(storage.clone());
    let owners = AylulOwners {
        agents: Agents::new(
            storage.clone(),
            providers.clone(),
            secrets,
            projects.clone(),
            None,
        ),
        providers,
        extensions: Extensions::new(storage.clone(), None),
        person: PersonSettings::new(storage.clone()),
    };
    let executable = std::env::current_exe().unwrap();
    let adapter = Arc::new(ClaudeAdapter::new(
        executable.clone(),
        executable.clone(),
        executable,
        "test".into(),
        "test".into(),
    ));
    let harness = Harness::with_owners(
        adapter,
        storage.clone(),
        SessionsConfig::default(),
        owners,
        Instructions::new(storage.clone()),
        Grants::new(storage.clone(), String::new()),
        Operations::new(storage.clone()),
        projects,
    );
    (tmp, storage, harness)
}

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

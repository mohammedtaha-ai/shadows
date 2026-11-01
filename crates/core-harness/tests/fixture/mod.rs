//! The harness every test in this crate opens: a fresh database, its owners and
//! a Claude Code adapter that is never launched.

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
use shadows_agent::claude::ClaudeAdapter;

pub async fn fixture() -> (tempfile::TempDir, Arc<Storage>, Arc<Harness>) {
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
            secrets.clone(),
            projects.clone(),
            None,
        ),
        providers,
        extensions: Extensions::new(storage.clone(), secrets, None),
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

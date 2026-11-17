//! Read the native choices without opening a Shadows thread.
use std::{path::Path, time::Duration};

use shadows_agent::acp::PermissionPolicy;
use shadows_agent::choices::Choice;
use shadows_agent::connection::{ConnectionError, ManagedConnection, SessionSetup, SessionStart};
use shadows_agent::policy;
use tokio::sync::Mutex;

use crate::{CatalogModel, Harness, HarnessCatalog, HarnessError, Sessions};

#[derive(Default)]
pub(super) struct Catalogs {
    claude: Probe,
    codex: Probe,
}

#[derive(Default)]
struct Probe {
    cached: Mutex<Option<HarnessCatalog>>,
    #[cfg(feature = "test-support")]
    attempts: std::sync::atomic::AtomicUsize,
    #[cfg(feature = "test-support")]
    fail_next: std::sync::atomic::AtomicBool,
}

impl Catalogs {
    fn for_kind(&self, kind: &str) -> Result<&Probe, HarnessError> {
        match kind {
            policy::CLAUDE_CODE => Ok(&self.claude),
            policy::CODEX => Ok(&self.codex),
            _ => Err(HarnessError::InvalidCommand(format!(
                "{kind} has no executor catalog"
            ))),
        }
    }
}

impl Harness {
    pub async fn catalog(&self, kind: &str) -> Result<HarnessCatalog, HarnessError> {
        let probe = self.catalogs.for_kind(kind)?;
        if kind == policy::CODEX && self.sessions.config.codex.is_none() {
            return Err(HarnessError::HarnessUnavailable(kind.into()));
        }
        let mut cached = probe.cached.lock().await;
        if let Some(catalog) = cached.as_ref() {
            return Ok(catalog.clone());
        }
        #[cfg(feature = "test-support")]
        {
            use std::sync::atomic::Ordering::Relaxed;
            probe.attempts.fetch_add(1, Relaxed);
            if probe.fail_next.swap(false, Relaxed) {
                return Err(failed("probe spawn failed"));
            }
        }
        let cwd = self
            .sessions
            .config
            .catalog_root
            .join("catalog-probe")
            .join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&cwd).await.map_err(failed)?;
        let models = self.sessions.probe_catalog(kind, &cwd).await;
        let _ = tokio::fs::remove_dir(&cwd).await;
        let catalog = HarnessCatalog {
            harness: kind.into(),
            models: models?,
        };
        *cached = Some(catalog.clone());
        Ok(catalog)
    }

    #[cfg(feature = "test-support")]
    pub fn catalog_probes_for_test(&self, kind: &str) -> usize {
        self.catalogs
            .for_kind(kind)
            .unwrap()
            .attempts
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    #[cfg(feature = "test-support")]
    pub fn fail_next_probe_for_test(&self, kind: &str) {
        self.catalogs
            .for_kind(kind)
            .unwrap()
            .fail_next
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Sessions {
    async fn probe_catalog(
        &self,
        kind: &str,
        cwd: &Path,
    ) -> Result<Vec<CatalogModel>, HarnessError> {
        let is_codex = kind == policy::CODEX;
        let spec = if is_codex {
            self.config
                .codex
                .as_ref()
                .ok_or_else(|| HarnessError::HarnessUnavailable(kind.into()))?
                .catalog_spec(cwd)
                .map_err(failed)?
        } else {
            self.adapter.process_spec(cwd)
        };
        let mut handle = shadows_process::spawn(spec)
            .map_err(|error| failed(format!("could not spawn {kind}: {error}")))?;
        let wait = self.config.setup_wait;
        let result = async {
            let connection = tokio::time::timeout(
                wait,
                ManagedConnection::open(&mut handle, is_codex, PermissionPolicy::Refuse, |_| {}),
            )
            .await
            .map_err(|_| timeout("initialization"))??;
            match connection {
                ManagedConnection::Codex(connection) => {
                    let choices = tokio::time::timeout(wait, connection.model_choices())
                        .await
                        .map_err(|_| timeout("model/list"))??;
                    Ok(choices
                        .into_iter()
                        .map(|(model, efforts)| catalog_model(model, efforts))
                        .collect())
                }
                claude => claude_choices(claude, cwd, wait).await,
            }
        }
        .await;
        let stopped = Self::stop_handle(&mut handle).await;
        let models = result.map_err(failed)?;
        stopped.map_err(|error| failed(format!("could not stop the {kind} probe: {error}")))?;
        Ok(models)
    }
}

async fn claude_choices(
    connection: ManagedConnection,
    cwd: &Path,
    wait: Duration,
) -> Result<Vec<CatalogModel>, ConnectionError> {
    let opened = tokio::time::timeout(
        wait,
        connection.start_session(cwd, SessionStart::New, &SessionSetup::default(), &[]),
    )
    .await
    .map_err(|_| timeout("session opening"))??;
    let mut offered = opened.options;
    let mut catalog = Vec::with_capacity(offered.models.len());
    for model in offered.models.clone() {
        if offered.current.model != model.id {
            let switched = tokio::time::timeout(
                wait,
                connection.set_option(&opened.session_id, &offered.ids.model, &model.id),
            )
            .await
            .map_err(|_| timeout(&format!("model {}", model.id)))?;
            match switched {
                Ok(next) => offered = next,
                // ACP reports this account-specific refusal as an RPC message.
                Err(ConnectionError::Rpc(message))
                    if message.contains("Usage credits are required for this model") =>
                {
                    tracing::debug!(model = %model.id, "catalog.model_requires_credits");
                    continue;
                }
                Err(error) => {
                    return Err(ConnectionError::Rpc(format!("model {}: {error}", model.id)));
                }
            }
        }
        if offered.current.model != model.id {
            return Err(ConnectionError::Rpc(format!(
                "model {} was not selected",
                model.id
            )));
        }
        catalog.push(catalog_model(model, offered.efforts.clone()));
    }
    Ok(catalog)
}

fn catalog_model(model: Choice, efforts: Vec<Choice>) -> CatalogModel {
    CatalogModel {
        id: model.id,
        label: model.label,
        description: model.description,
        efforts,
    }
}

fn failed(error: impl std::fmt::Display) -> HarnessError {
    HarnessError::HarnessStartFailed(error.to_string())
}

fn timeout(step: &str) -> ConnectionError {
    ConnectionError::Rpc(format!("catalog probe timed out during {step}"))
}

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

/// How long a read catalog stands before the next read probes again: Codex's own model-list
/// cache lives as long. A CLI's list changes under a running daemon (a new model, an update,
/// or Codex answering its built-in list while its refresh fails), and one answer kept for the
/// daemon's life once hid `gpt-6.2-sol` from every executor choice.
const CATALOG_TTL: Duration = Duration::from_secs(300);

#[derive(Default)]
struct Probe {
    /// The last catalog read and when; an older one than CATALOG_TTL is read again.
    cached: Mutex<Option<(HarnessCatalog, tokio::time::Instant)>>,
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
    /// Catalogs the Planner can currently use.
    pub async fn executor_catalogs(&self) -> Vec<HarnessCatalog> {
        let (claude, codex) = tokio::join!(
            self.catalog(policy::CLAUDE_CODE),
            self.catalog(policy::CODEX)
        );
        [claude, codex].into_iter().filter_map(Result::ok).collect()
    }

    pub async fn catalog(&self, kind: &str) -> Result<HarnessCatalog, HarnessError> {
        let probe = self.catalogs.for_kind(kind)?;
        if kind == policy::CODEX && self.sessions.config.codex.is_none() {
            return Err(HarnessError::HarnessUnavailable(kind.into()));
        }
        let mut cached = probe.cached.lock().await;
        let now = tokio::time::Instant::now();
        if let Some((catalog, read_at)) = cached.as_ref()
            && now.duration_since(*read_at) < CATALOG_TTL
        {
            return Ok(catalog.clone());
        }
        match self.probe(probe, kind).await {
            Ok(catalog) => {
                *cached = Some((catalog.clone(), now));
                Ok(catalog)
            }
            // A failed reading keeps the last list for another while rather than leave the
            // menus empty; it is tried again once that while has passed.
            Err(error) => match cached.as_mut() {
                Some((catalog, read_at)) => {
                    tracing::warn!(kind, %error, "harness.catalog_reread_failed");
                    *read_at = now;
                    Ok(catalog.clone())
                }
                None => Err(error),
            },
        }
    }

    async fn probe(&self, probe: &Probe, kind: &str) -> Result<HarnessCatalog, HarnessError> {
        #[cfg(feature = "test-support")]
        {
            use std::sync::atomic::Ordering::Relaxed;
            probe.attempts.fetch_add(1, Relaxed);
            if probe.fail_next.swap(false, Relaxed) {
                return Err(failed("probe spawn failed"));
            }
        }
        #[cfg(not(feature = "test-support"))]
        let _ = probe;
        let cwd = self
            .sessions
            .config
            .catalog_root
            .join("catalog-probe")
            .join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&cwd).await.map_err(failed)?;
        let models = self.sessions.probe_catalog(kind, &cwd).await;
        let _ = tokio::fs::remove_dir(&cwd).await;
        Ok(HarnessCatalog {
            harness: kind.into(),
            models: models?,
        })
    }

    /// Ages the read catalog of `kind` past CATALOG_TTL, so the next read probes again.
    #[cfg(feature = "test-support")]
    pub async fn age_catalog_for_test(&self, kind: &str) {
        let probe = self.catalogs.for_kind(kind).unwrap();
        if let Some((_, read_at)) = probe.cached.lock().await.as_mut() {
            *read_at -= CATALOG_TTL;
        }
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
        let checked = if is_codex {
            Some(self.codex_checked().await?)
        } else {
            None
        };
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
                ManagedConnection::open(
                    &mut handle,
                    checked.as_ref().map(|checked| checked.version.as_str()),
                    PermissionPolicy::Refuse,
                    |_| {},
                ),
            )
            .await
            .map_err(|_| timeout("initialization"))?;
            if let Err(error) = &connection {
                self.forget_codex_check_if_changed(error).await;
            }
            let connection = connection?;
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

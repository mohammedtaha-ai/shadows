pub mod config;
pub mod launch;
use config::{read_json, write_json, Failure, LaunchConfig};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadyConnection {
    pub origin: String,
    pub configuration_id: String,
    pub runtime_id: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub revision: u64,
    pub state: &'static str,
    pub configuration_id: Option<String>,
    pub origin: Option<String>,
    pub error: Option<String>,
}
type Notify = Arc<dyn Fn(ConnectionView) + Send + Sync>;
struct Inner {
    config: Option<LaunchConfig>,
    epoch: u64,
    ready: Option<ReadyConnection>,
    view: ConnectionView,
}
pub struct Connection {
    root: PathBuf,
    inner: Mutex<Inner>,
    notifier: Mutex<Option<Notify>>,
}
impl Connection {
    pub fn new(root: PathBuf) -> Arc<Self> {
        let loaded = read_json::<LaunchConfig>(&root.join("desktop-core.json")).and_then(|value| {
            value
                .map(|config| {
                    let normalized = config.clone().normalized()?;
                    if normalized.configuration_id != config.configuration_id {
                        return Err("core_config_invalid");
                    }
                    Ok(normalized)
                })
                .transpose()
        });
        let (config, error) = match loaded {
            Ok(config) => (config, None),
            Err(error) => (None, Some(error.into())),
        };
        let view = ConnectionView {
            revision: 0,
            state: if error.is_some() {
                "failed"
            } else if config.is_some() {
                "starting"
            } else {
                "unconfigured"
            },
            configuration_id: config.as_ref().map(|c| c.configuration_id.clone()),
            origin: None,
            error,
        };
        Arc::new(Self {
            root,
            inner: Mutex::new(Inner {
                config,
                epoch: 0,
                ready: None,
                view,
            }),
            notifier: Mutex::new(None),
        })
    }
    pub fn set_notifier(&self, notify: Notify) {
        *self.notifier.lock().unwrap() = Some(notify);
    }
    fn notify(&self) {
        let notify = self.notifier.lock().unwrap().clone();
        if let Some(notify) = notify {
            notify(self.snapshot());
        }
    }
    pub fn snapshot(&self) -> ConnectionView {
        self.inner.lock().unwrap().view.clone()
    }
    pub fn configured(&self) -> Option<LaunchConfig> {
        self.inner.lock().unwrap().config.clone()
    }
    pub fn ready(&self) -> Option<ReadyConnection> {
        self.inner.lock().unwrap().ready.clone()
    }
    pub fn save(&self, config: LaunchConfig) -> Result<bool, Failure> {
        let config = config.normalized()?;
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.config.as_ref() == Some(&config) {
                return Ok(false);
            }
            write_json(&self.root.join("desktop-core.json"), &config)?;
            inner.epoch += 1;
            inner.ready = None;
            inner.view = ConnectionView {
                revision: inner.view.revision + 1,
                state: "starting",
                configuration_id: Some(config.configuration_id.clone()),
                origin: None,
                error: None,
            };
            inner.config = Some(config);
        }
        self.notify();
        Ok(true)
    }
    fn begin(&self) -> Option<(u64, LaunchConfig)> {
        let result = {
            let mut inner = self.inner.lock().unwrap();
            let config = inner.config.clone()?;
            inner.epoch += 1;
            inner.ready = None;
            inner.view.revision += 1;
            inner.view.state = "starting";
            inner.view.origin = None;
            inner.view.error = None;
            (inner.epoch, config)
        };
        self.notify();
        Some(result)
    }
    fn finish(&self, epoch: u64, result: Result<ReadyConnection, Failure>) {
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.epoch != epoch {
                return;
            }
            let result = result.and_then(|ready| {
                if inner.config.as_ref().is_some_and(|config| {
                    config.configuration_id == ready.configuration_id
                        && config.origin == ready.origin
                }) {
                    Ok(ready)
                } else {
                    Err("core_identity_mismatch")
                }
            });
            inner.view.revision += 1;
            match result {
                Ok(ready) => {
                    inner.view.state = "ready";
                    inner.view.origin = Some(ready.origin.clone());
                    inner.view.error = None;
                    inner.ready = Some(ready);
                }
                Err(error) => {
                    inner.view.state = "failed";
                    inner.view.origin = None;
                    inner.view.error = Some(error.into());
                    inner.ready = None;
                }
            }
        }
        self.notify();
    }
    pub fn retry(&self) -> ConnectionView {
        if let Some((epoch, config)) = self.begin() {
            let result = launch::ensure_running(&config, &self.root, Duration::from_secs(30));
            self.finish(epoch, result);
        }
        self.snapshot()
    }
}
#[cfg(test)]
mod connection_tests;

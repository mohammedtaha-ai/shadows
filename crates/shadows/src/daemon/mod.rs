//! Compose optional planning beside the independently owned Node assistant.
pub mod bridge;
mod descriptor;
pub mod http;

use serde::Deserialize;
use shadows_core::{AppCore, StartConfig};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Semaphore};

pub struct Config {
    pub node: PathBuf,
    pub entry: PathBuf,
    pub config: PathBuf,
    pub descriptor: PathBuf,
    pub bind: SocketAddr,
    pub planner_config: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlannerConfig {
    db_path: PathBuf,
    node_path: PathBuf,
    adapter_path: PathBuf,
    harness_path: PathBuf,
    allowed_origins: Vec<String>,
    #[serde(default)]
    aylul: Option<crate::aylul::AylulSection>,
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    if !config.bind.ip().is_loopback() {
        anyhow::bail!("product daemon requires a loopback listener");
    }
    for path in [&config.node, &config.entry, &config.config] {
        crate::config::harness_path(path)?;
    }
    let _owner = descriptor::claim(&config.descriptor)?;
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let address = listener.local_addr()?;
    let (bridge, installation_id) =
        bridge::Bridge::start(&config.node, &config.entry, &config.config).await?;
    let bridge = Arc::new(Mutex::new(bridge));
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let mut router = http::router(http::StateData {
        bridge: bridge.clone(),
        token: token.clone(),
        admission: Arc::new(Semaphore::new(8)),
    });
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let core = if let Some(path) = config.planner_config {
        let planner: PlannerConfig =
            serde_json::from_slice(&std::fs::read(crate::config::harness_path(&path)?)?)?;
        if !planner.db_path.is_absolute() {
            anyhow::bail!("planner database path must be absolute");
        }
        for path in [
            &planner.node_path,
            &planner.adapter_path,
            &planner.harness_path,
        ] {
            crate::config::harness_path(path)?;
        }
        let origins = planner
            .allowed_origins
            .iter()
            .map(|origin| crate::config::allowed_origin(origin))
            .collect::<Result<Vec<_>, _>>()?;
        let prepared = crate::aylul::prepare(planner.aylul)?;
        let aylul = prepared;
        let secret_key = aylul.as_ref().and_then(|_| {
            crate::master_key::load_or_create()
                .map_err(|error| {
                    tracing::warn!(%error, "secrets.master_key_unavailable");
                })
                .ok()
        });
        let core = AppCore::start(
            &StartConfig {
                secret_key,
                db_path: planner.db_path,
                node_path: planner.node_path,
                adapter_path: planner.adapter_path,
                harness_path: planner.harness_path,
                aylul: aylul.clone(),
            },
            format!("http://{address}/mcp"),
        )
        .await?;
        crate::aylul::ensure_project(&core, aylul.as_ref()).await?;

        router = router.merge(crate::cli::router(shadows_http::AppState {
            core: core.clone(),
            allowed_origins: origins,
            shutdown,
        }));
        Some(core)
    } else {
        None
    };
    descriptor::write(
        &config.descriptor,
        &descriptor::Descriptor {
            origin: format!("http://{address}"),
            installation_id,
            token,
        },
    )?;
    println!("shadows daemon listening on http://{address}");
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            if let Some(core) = core {
                let _ = core
                    .shut_down(Duration::from_secs(10), std::future::pending())
                    .await;
            }
            bridge.lock().await.stop();
            stopping.send_replace(true);
        })
        .await?;
    let _ = std::fs::remove_file(config.descriptor);
    Ok(())
}

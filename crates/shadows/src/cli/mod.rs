//! One job: the daemon's entry point — bind, start the application, serve it.

pub mod args;

use std::sync::Arc;
use std::time::Duration;

use crate::config::Config;
use shadows_core::{AppCore, StartConfig};
use shadows_http::AppState;

/// Binds, prints exactly one address, and serves. Spec §1.0: it never opens a
/// browser. The user chooses which browser to use.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    // Bound first (§14.4): a grant's `claude mcp add` names the address
    // actually bound, which differs from `config.bind` when that asks for port
    // 0, and every Planner session opens with that `/mcp` (§13.8). A daemon
    // that cannot bind stops here, before recovery touches anything.
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let addr = listener.local_addr()?;
    let mcp_url = format!("http://{addr}/mcp");
    let start = StartConfig {
        db_path: config.db_path.clone(),
        node_path: config.node_path.clone(),
        adapter_path: config.adapter_path.clone(),
        harness_path: config.harness_path.clone(),
    };
    let core = AppCore::start(&start, mcp_url).await?;
    let (stopping, shutdown) = tokio::sync::watch::channel(false);
    let state = AppState {
        core: core.clone(),
        allowed_origins: config.allowed_origins.clone(),
        shutdown,
    };

    println!("shadows serve listening on http://{addr}");
    if let Some(path) = &config.debug_log {
        println!("shadows serve debug log: {}", path.display());
    }

    // Spec §8.5: shutdown reuses the cancellation path. There is no drain mode.
    // The listener keeps accepting until this future returns — axum stops
    // accepting only then — so a turn requested meanwhile is refused by the
    // closed registry, not by the socket.
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            if let Err(error) = tokio::signal::ctrl_c().await {
                // Without a signal there is no way to be asked to stop, and
                // stopping now would end a daemon nobody asked to end. Serve
                // on; a killed daemon's turns are recovered at next startup.
                tracing::error!(%error, "shutdown.signal_unavailable");
                std::future::pending::<()>().await;
            }
            tracing::info!("stop signal received; cancelling this runtime's operations");
            let second_signal = async {
                // An error here means no second signal can ever arrive, which
                // is the same as one that has not arrived yet.
                if tokio::signal::ctrl_c().await.is_err() {
                    std::future::pending::<()>().await;
                }
            };
            match core.shut_down(CONFIRMATION_BOUND, second_signal).await {
                Ok(kind) => tracing::info!(stop_kind = ?kind, "shutdown.recorded"),
                Err(error) => {
                    tracing::error!(%error, "shutdown.unrecorded: the stop could not be written")
                }
            }
            // Last: open live streams end here, so a graceful HTTP shutdown
            // is not left waiting on a response that never finishes.
            stopping.send_replace(true);
        })
        .await?;
    Ok(())
}

/// The daemon's whole router: the HTTP routes with `/mcp` mounted (spec
/// §14.5). `/mcp` reaches the same application as the HTTP routes, because
/// both are built from the `AppState`'s one `AppCore`. `serve` serves it, and
/// the tests drive it, so both run one wiring.
pub fn router(state: AppState) -> axum::Router {
    let mcp = shadows_mcp::service(Arc::clone(&state.core));
    shadows_http::router(state, mcp)
}

/// How long shutdown waits for every owned operation to be confirmed terminal
/// before it records `Escalated` instead of `Graceful` (§8.5). A terminated
/// tree is reaped, and its outcome written, in milliseconds; the bound exists
/// for the turn that never confirms, so a daemon told once to stop still ends.
const CONFIRMATION_BOUND: Duration = Duration::from_secs(10);

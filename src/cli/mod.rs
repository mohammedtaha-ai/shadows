use std::net::SocketAddr;

use crate::config::Config;

/// Binds, prints exactly one address, and serves. Spec §1.0: it never opens a
/// browser. The user chooses which browser to use.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let addr: SocketAddr = listener.local_addr()?;
    println!("shadows serve listening on http://{addr}");
    tracing::info!(%addr, "daemon bound");

    let app = axum::Router::new().route("/health", axum::routing::get(|| async { "ok" }));
    axum::serve(listener, app).await?;
    Ok(())
}

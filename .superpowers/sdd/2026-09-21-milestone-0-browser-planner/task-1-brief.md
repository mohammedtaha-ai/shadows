## Task 1: Crate scaffold, `shadows serve`, structured tracing

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/config.rs`, `src/tracing.rs`, `src/cli/mod.rs`, `src/error.rs`
- Create: `tests/serve_smoke.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `shadows::config::Config { db_path: PathBuf, bind: SocketAddr, harness_path: PathBuf }`; `shadows::tracing::init(verbose: bool)`; `shadows::cli::serve(config: Config) -> anyhow::Result<()>`; `shadows::error::{AppFailure, ErrorCode, FailureClass, RetryClass}`.

- [ ] **Step 1: Write the failing test**

`tests/serve_smoke.rs`:

```rust
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

/// `shadows serve` must print exactly one local address and must not open a
/// browser. Spec §1.0 and §11.1 both require the daemon to stop at printing.
#[test]
fn serve_prints_one_local_address_and_does_not_open_a_browser() {
    let exe = env!("CARGO_BIN_EXE_shadows");
    let tmp = tempfile::tempdir().unwrap();
    let mut child = Command::new(exe)
        .arg("serve")
        .arg("--db")
        .arg(tmp.path().join("shadows.sqlite3"))
        .arg("--bind")
        .arg("127.0.0.1:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("daemon should start");

    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let first = lines.next().expect("expected a line").unwrap();

    assert!(
        first.starts_with("shadows serve listening on http://127.0.0.1:"),
        "unexpected first line: {first}"
    );

    child.kill().unwrap();
    child.wait().unwrap();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test serve_smoke`
Expected: FAIL — the crate does not exist yet, so this does not compile.

- [ ] **Step 3: Write `Cargo.toml`**

```toml
[package]
name = "shadows"
version = "0.1.0"
edition = "2024"
rust-version = "1.94"
publish = false

[dependencies]
anyhow = "1"
axum = "0.8"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.9", features = ["runtime-tokio", "sqlite", "migrate"] }
thiserror = "2"
time = { version = "0.3", features = ["formatting", "parsing", "macros"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
tower-http = { version = "0.6", features = ["trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
uuid = { version = "1", features = ["v4", "serde"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4: Write `src/error.rs`**

This is spec §3.2 and §3.4 made concrete. `AppFailure` is public-safe; the causal chain stays in `FailureReport` and cannot accidentally serialize.

```rust
use std::fmt;

/// Stable codes clients pattern-match on. Never match on human text.
/// Spec §3.4. `Blocked`/`Rejected` are domain outcomes and never appear here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ProcessSpawnFailed,
    ProcessTerminated,
    StorageUnavailable,
    StorageMigrationFailed,
    StorageConstraintViolation,
    CommandConflict,
    IdempotencyKeyRequired,
    InvalidCommand,
    InvalidCursor,
    AgentAuthFailed,
    AgentUnsupportedProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureClass {
    Client,
    Infrastructure,
    Agent,
    Storage,
}

/// Classifying a failure as retryable does not authorize an automatic retry.
/// Spec §3.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetryClass {
    Never,
    Immediate,
    Backoff,
    AfterReconfiguration,
    AfterUserAction,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppFailure {
    pub code: ErrorCode,
    pub class: FailureClass,
    pub retry: RetryClass,
    pub public_details: String,
}

impl fmt::Display for AppFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.public_details)
    }
}

impl std::error::Error for AppFailure {}

/// The internal causal chain. Deliberately not `Serialize` — spec §3.2 requires
/// that it cannot accidentally reach a client.
#[derive(Debug)]
pub struct FailureReport {
    pub failure: AppFailure,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
```

- [ ] **Step 5: Write `src/config.rs`**

`harness_path` is explicit, never resolved from `PATH` — spec §1.4, and the reason is in the harness evidence report.

```rust
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub bind: SocketAddr,
    /// Spec §1.4: resolved from configuration, never from `PATH`. This machine
    /// carries more than one `claude-code` installation at different versions.
    pub harness_path: PathBuf,
}
```

- [ ] **Step 6: Write `src/tracing.rs`**

```rust
/// Spec §8.7. Durable transition logs are emitted only after their transaction
/// commits; that discipline lives at the call sites, not here.
pub fn init(verbose: bool) {
    use tracing_subscriber::{EnvFilter, fmt};
    let default = if verbose { "shadows=debug,info" } else { "shadows=info,warn" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default));
    fmt().with_env_filter(filter).with_target(true).init();
}
```

- [ ] **Step 7: Write `src/lib.rs`**

```rust
pub mod cli;
pub mod config;
pub mod error;
pub mod tracing;
```

- [ ] **Step 8: Write `src/cli/mod.rs`**

```rust
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
```

- [ ] **Step 9: Write `src/main.rs`**

`anyhow` is used here and only here — spec §3.6 limits it to the bootstrap boundary.

```rust
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use shadows::config::Config;

#[derive(Parser)]
#[command(name = "shadows")]
struct Cli {
    #[arg(long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the local daemon.
    Serve {
        #[arg(long, default_value = "shadows.sqlite3")]
        db: PathBuf,
        #[arg(long, default_value = "127.0.0.1:4318")]
        bind: SocketAddr,
        /// Path to the Claude Code executable. Spec §1.4 forbids PATH lookup.
        #[arg(long, default_value = "claude")]
        harness: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    shadows::tracing::init(cli.verbose);
    match cli.command {
        Commands::Serve { db, bind, harness } => {
            shadows::cli::serve(Config { db_path: db, bind, harness_path: harness }).await
        }
    }
}
```

- [ ] **Step 10: Run test to verify it passes**

Run: `cargo test --test serve_smoke`
Expected: PASS.

Then run: `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
Expected: both clean.

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml Cargo.lock src tests
git commit -m "feat: shadows serve binds, prints one address, and opens no browser"
```

---


/// Spec §8.7. Durable transition logs are emitted only after their transaction
/// commits; that discipline lives at the call sites, not here.
pub fn init(verbose: bool) {
    use tracing_subscriber::{EnvFilter, fmt};
    let default = if verbose {
        "shadows=debug,info"
    } else {
        "shadows=info,warn"
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default));
    // Diagnostics go to stderr, never stdout. Spec §1.0 gives stdout one job —
    // printing the single local address `shadows serve` binds — and Task 11 put
    // the first log line (startup recovery) before that print. A subscriber
    // writing to stdout would make the daemon's one promised output the second
    // or tenth line, depending on how much recovery had to do.
    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_writer(std::io::stderr)
        .init();
}

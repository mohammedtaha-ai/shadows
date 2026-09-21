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
    fmt().with_env_filter(filter).with_target(true).init();
}

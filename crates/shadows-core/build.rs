// `sqlx::migrate!` embeds `migrations/` at compile time, but on stable Rust it
// cannot tell Cargo to watch that directory. Without this line, adding or
// changing a migration with no accompanying `src/` change leaves a build that
// silently embeds the old set. This is the build script sqlx documents for it.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}

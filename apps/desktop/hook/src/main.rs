use std::io::{Read, Write};

/// Every path exits 0 with no stdout beyond the sidecar's answer: a panic is
/// silenced and caught, and a closed stdout is ignored.
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let _ = std::panic::catch_unwind(run);
    std::process::exit(0);
}

fn run() {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return;
    }
    let endpoint = std::env::var_os("LOCALAPPDATA")
        .map(|dir| {
            std::path::PathBuf::from(dir)
                .join("AYLUL")
                .join("hook.json")
        })
        .and_then(|path| aylul_hook::read_endpoint(&path));
    let project_dir = std::env::var("CLAUDE_PROJECT_DIR")
        .ok()
        .filter(|dir| !dir.is_empty());
    let open = |pipe: &str| {
        let pipe = pipe.to_owned();
        aylul_hook::connect_retrying(
            || {
                std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&pipe)
            },
            50,
            std::time::Duration::from_millis(20),
        )
    };
    if let Some(output) = aylul_hook::run(&input, endpoint, project_dir, open) {
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(output.as_bytes());
        let _ = stdout.flush();
    }
}

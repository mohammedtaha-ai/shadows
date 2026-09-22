//! Test-support binary. Not product code. It exists so the containment test
//! has a real daemon -> child -> grandchild hierarchy to terminate.

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--read-stdin") {
        let mut buf = String::new();
        // With stdin closed this returns Ok(0) immediately. With an inherited
        // console stdin it blocks, and the test times out.
        let _ = std::io::stdin().read_to_string(&mut buf);
        return;
    }

    let spawn_grandchild = args.iter().any(|a| a == "--spawn-grandchild");
    let exit_after_spawn = args.iter().any(|a| a == "--spawn-grandchild-and-exit");
    if spawn_grandchild || exit_after_spawn {
        let me = std::env::current_exe().expect("current exe");
        // The grandchild sleeps 600s; the containment test kills the whole
        // tree through the Job Object. Calling `.wait()` here would deadlock
        // the probe and defeat the test, so the lint is explicitly allowed.
        #[allow(clippy::zombie_processes)]
        let grandchild = std::process::Command::new(me)
            .arg("--sleep")
            .spawn()
            .expect("grandchild should spawn");
        println!("grandchild={}", grandchild.id());
        use std::io::Write;
        std::io::stdout().flush().unwrap();
        if exit_after_spawn {
            return;
        }
    }

    // Both the child and the grandchild end up here and sleep until killed.
    std::thread::sleep(std::time::Duration::from_secs(600));
}

//! The executor's commit procedure on a folder other executors share: stage
//! its new paths by exact name, then `git --literal-pathspecs commit --only`
//! with its own exact paths. Another executor's staged content stays staged
//! and out of the commit. Real Git in a temporary folder.

use std::path::Path;

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn write(root: &Path, path: &str, text: &str) {
    let at = root.join(path);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(at, text).unwrap();
}

/// A repository with `files` committed.
fn repository(files: &[&str]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "runs@example.invalid"]);
    git(root, &["config", "user.name", "Runs"]);
    git(root, &["config", "core.autocrlf", "false"]);
    for path in files {
        write(root, path, "start\n");
        git(root, &["add", "--", path]);
    }
    git(root, &["commit", "-q", "-m", "start"]);
    tmp
}

/// The procedure the executor's instructions give: new paths staged by
/// exact name first, then one commit of exactly `own`.
fn commit_own(root: &Path, new: &[&str], own: &[&str], message: &str) -> String {
    if !new.is_empty() {
        let mut add = vec!["--literal-pathspecs", "add", "--"];
        add.extend_from_slice(new);
        git(root, &add);
    }
    let mut commit = vec![
        "--literal-pathspecs",
        "commit",
        "-q",
        "--only",
        "-m",
        message,
        "--",
    ];
    commit.extend_from_slice(own);
    git(root, &commit);
    git(root, &["rev-parse", "HEAD"]).trim().to_string()
}

/// The paths a commit changed, with their status letters, sorted.
fn changed(root: &Path, sha: &str) -> Vec<String> {
    let out = git(
        root,
        &["show", "--no-renames", "--name-status", "--format=", sha],
    );
    let mut lines: Vec<String> = out.lines().map(|l| l.replace('\t', " ")).collect();
    lines.sort();
    lines
}

/// What the index holds staged against HEAD, sorted.
fn staged(root: &Path) -> Vec<String> {
    let out = git(root, &["diff", "--cached", "--no-renames", "--name-status"]);
    let mut lines: Vec<String> = out.lines().map(|l| l.replace('\t', " ")).collect();
    lines.sort();
    lines
}

#[test]
fn only_commit_preserves_foreign_staged_paths() {
    let tmp = repository(&["web/a.ts", "lib/b.rs"]);
    let root = tmp.path();
    // Executor B stages its change, then executor A commits its own.
    write(root, "lib/b.rs", "B's change\n");
    git(root, &["add", "--", "lib/b.rs"]);
    write(root, "web/a.ts", "A's change\n");
    let a = commit_own(root, &[], &["web/a.ts"], "A");
    assert_eq!(changed(root, &a), ["M web/a.ts"]);
    assert_eq!(
        staged(root),
        ["M lib/b.rs"],
        "B's staged change stays staged"
    );
    assert_eq!(
        git(root, &["show", ":lib/b.rs"]),
        "B's change\n",
        "B's staged content is intact"
    );
    let b = commit_own(root, &[], &["lib/b.rs"], "B");
    assert_eq!(changed(root, &b), ["M lib/b.rs"]);
    assert!(staged(root).is_empty());
}

#[test]
fn new_deleted_and_renamed_paths_commit_only_their_own_changes() {
    let tmp = repository(&["web/old.ts", "web/gone.ts", "lib/b.rs"]);
    let root = tmp.path();
    // Executor B stages a new file of its own before A commits.
    write(root, "lib/new_b.rs", "B's new file\n");
    git(root, &["add", "--", "lib/new_b.rs"]);
    // A creates, deletes and renames inside web/.
    write(root, "web/new.ts", "A's new file\n");
    std::fs::remove_file(root.join("web/gone.ts")).unwrap();
    std::fs::rename(root.join("web/old.ts"), root.join("web/renamed.ts")).unwrap();
    let a = commit_own(
        root,
        &["web/new.ts", "web/renamed.ts"],
        &["web/new.ts", "web/gone.ts", "web/old.ts", "web/renamed.ts"],
        "A",
    );
    assert_eq!(
        changed(root, &a),
        [
            "A web/new.ts",
            "A web/renamed.ts",
            "D web/gone.ts",
            "D web/old.ts"
        ]
    );
    assert_eq!(
        staged(root),
        ["A lib/new_b.rs"],
        "B's new file stays staged"
    );
    assert_eq!(git(root, &["show", ":lib/new_b.rs"]), "B's new file\n");
    assert!(git(root, &["status", "--short", "--", "web"]).is_empty());
}

#[test]
fn literal_pathspecs_do_not_select_other_files() {
    let tmp = repository(&["web/[id].ts", "web/i.ts", "web/d.ts", "web/ab.ts"]);
    let root = tmp.path();
    for path in ["web/i.ts", "web/d.ts", "web/ab.ts"] {
        write(root, path, "someone else's change\n");
    }
    git(root, &["add", "--", "web/ab.ts"]);
    write(root, "web/[id].ts", "A's change\n");
    let a = commit_own(root, &[], &["web/[id].ts"], "A");
    assert_eq!(changed(root, &a), ["M web/[id].ts"]);
    assert_eq!(staged(root), ["M web/ab.ts"]);
    let left = git(root, &["status", "--short"]);
    for path in ["web/i.ts", "web/d.ts", "web/ab.ts"] {
        assert!(left.contains(path), "{path} untouched: {left}");
    }
}

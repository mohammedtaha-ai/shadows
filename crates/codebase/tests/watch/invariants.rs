use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use notify::EventKind;
use notify::event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RenameMode};
use tokio::sync::mpsc;

use super::{Seen, forward};

/// Linux reports every read: none may reach the worker, or a scan's own
/// reads start the next scan, forever. A write still does.
#[test]
fn a_read_is_not_a_change() {
    let dir = Path::new("/project");
    let (seen, mut rx) = mpsc::channel(8);
    let lost = AtomicBool::new(false);
    let event = |kind| Ok(notify::Event::new(kind).add_path(dir.join("src/a.rs")));
    for kind in [
        EventKind::Access(AccessKind::Open(AccessMode::Any)),
        EventKind::Access(AccessKind::Close(AccessMode::Read)),
    ] {
        forward(dir, &seen, &lost, &|_: &str| {}, event(kind));
    }
    assert!(rx.try_recv().is_err());
    assert!(!lost.load(Ordering::Relaxed));
    for kind in [
        EventKind::Modify(ModifyKind::Data(DataChange::Any)),
        EventKind::Access(AccessKind::Close(AccessMode::Write)),
    ] {
        forward(dir, &seen, &lost, &|_: &str| {}, event(kind));
        assert!(matches!(rx.try_recv(), Ok(Seen::Path(p)) if p == "src/a.rs"));
    }
}

/// Windows reports a folder as modified when a file in it is only read: the
/// holds do not take the folder for a changed file, and the index still sees it.
/// A folder made or renamed is still a change.
#[test]
fn a_folder_modified_in_place_is_not_a_hold_change() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path();
    std::fs::create_dir(dir.join("notes")).unwrap();
    let (seen, mut rx) = mpsc::channel(8);
    let lost = AtomicBool::new(false);
    let told = std::sync::Mutex::new(Vec::new());
    let saw = |rel: &str| told.lock().unwrap().push(rel.to_owned());
    let event = |kind| Ok(notify::Event::new(kind).add_path(dir.join("notes")));
    forward(
        dir,
        &seen,
        &lost,
        &saw,
        event(EventKind::Modify(ModifyKind::Any)),
    );
    assert!(told.lock().unwrap().is_empty());
    assert!(matches!(rx.try_recv(), Ok(Seen::Path(p)) if p == "notes"));
    for kind in [
        EventKind::Create(CreateKind::Folder),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)),
    ] {
        forward(dir, &seen, &lost, &saw, event(kind));
    }
    assert_eq!(*told.lock().unwrap(), ["notes", "notes"]);
}

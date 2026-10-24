use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use notify::EventKind;
use notify::event::{AccessKind, AccessMode, DataChange, ModifyKind};
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
        forward(dir, &seen, &lost, event(kind));
    }
    assert!(rx.try_recv().is_err());
    assert!(!lost.load(Ordering::Relaxed));
    for kind in [
        EventKind::Modify(ModifyKind::Data(DataChange::Any)),
        EventKind::Access(AccessKind::Close(AccessMode::Write)),
    ] {
        forward(dir, &seen, &lost, event(kind));
        assert!(matches!(rx.try_recv(), Ok(Seen::Path(p)) if p == "src/a.rs"));
    }
}

use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
#[derive(Default)]
struct Fake {
    enabled: AtomicBool,
    refuse: AtomicBool,
    refuse_restore: AtomicBool,
    calls: Mutex<Vec<bool>>,
}
impl LoginItem for Arc<Fake> {
    fn is_enabled(&self) -> Result<bool, ()> {
        Ok(self.enabled.load(Ordering::SeqCst))
    }
    fn set_enabled(&self, enabled: bool) -> Result<(), ()> {
        self.calls.lock().unwrap().push(enabled);
        if self.refuse.load(Ordering::SeqCst)
            || (!enabled && self.refuse_restore.load(Ordering::SeqCst))
        {
            return Err(());
        }
        self.enabled.store(enabled, Ordering::SeqCst);
        Ok(())
    }
}
fn setup() -> (tempfile::TempDir, DesktopPreferences, Arc<Fake>) {
    let root = tempfile::tempdir().unwrap();
    let login = Arc::new(Fake::default());
    let store = DesktopPreferences::new(root.path().to_path_buf(), login.clone());
    (root, store, login)
}
#[test]
fn desktop_preferences_fresh_install_preserves_close_to_tray() {
    let (root, store, login) = setup();
    assert_eq!(
        serde_json::to_value(store.read().unwrap()).unwrap(),
        serde_json::json!({"launchAtLogin":false,"closeToTray":true})
    );
    assert_eq!(store.close_action(false), CloseAction::Hide);
    assert!(login.calls.lock().unwrap().is_empty());
    assert!(!root.path().join("desktop-preferences-v1.json").exists());
}
#[test]
fn desktop_preferences_partial_update_survives_restart() {
    let (root, store, login) = setup();
    let first = store.set(Some(true), None).unwrap();
    assert!(first.launch_at_login && first.close_to_tray);
    let second = store.set(None, Some(false)).unwrap();
    assert!(second.launch_at_login && !second.close_to_tray);
    let path = root.path().join("desktop-preferences-v1.json");
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
        serde_json::json!({"launchAtLogin":true,"closeToTray":false})
    );
    assert_eq!(store.set(None, None).unwrap(), second);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let reopened = DesktopPreferences::new(root.path().to_path_buf(), login.clone());
    assert_eq!(reopened.read().unwrap(), second);
    assert_eq!(reopened.close_action(false), CloseAction::Exit);
    assert_eq!(*login.calls.lock().unwrap(), vec![true]);
}
#[test]
fn desktop_preferences_reports_external_login_change() {
    let (_root, store, login) = setup();
    login.enabled.store(true, Ordering::SeqCst);
    assert!(store.read().unwrap().launch_at_login);
    assert!(login.calls.lock().unwrap().is_empty());
}
#[test]
fn desktop_preferences_os_refusal_preserves_saved_close_policy() {
    let (root, store, login) = setup();
    store.set(None, Some(true)).unwrap();
    let before = std::fs::read(root.path().join("desktop-preferences-v1.json")).unwrap();
    login.refuse.store(true, Ordering::SeqCst);
    assert_eq!(
        store.set(Some(true), Some(false)).unwrap_err(),
        "desktop_preferences_autostart_failed"
    );
    assert_eq!(store.close_action(false), CloseAction::Hide);
    assert!(!store.read().unwrap().launch_at_login);
    assert_eq!(
        std::fs::read(root.path().join("desktop-preferences-v1.json")).unwrap(),
        before
    );
}
#[test]
fn desktop_preferences_corrupt_or_unwritable_store_is_not_overwritten() {
    let (root, _, login) = setup();
    let path = root.path().join("desktop-preferences-v1.json");
    std::fs::write(&path, b"broken").unwrap();
    let store = DesktopPreferences::new(root.path().to_path_buf(), login.clone());
    assert_eq!(store.read().unwrap_err(), "desktop_preferences_load_failed");
    assert_eq!(
        store.set(Some(true), Some(false)).unwrap_err(),
        "desktop_preferences_load_failed"
    );
    assert_eq!(store.close_action(false), CloseAction::Hide);
    assert_eq!(std::fs::read(&path).unwrap(), b"broken");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let store = DesktopPreferences::new(root.path().join("new"), login.clone());
    std::fs::create_dir(root.path().join("new")).unwrap();
    std::fs::create_dir(root.path().join("new/desktop-preferences-v1.json")).unwrap();
    assert_eq!(
        store.set(Some(true), Some(false)).unwrap_err(),
        "desktop_preferences_save_failed"
    );
    assert_eq!(store.close_action(false), CloseAction::Hide);
    assert!(!login.enabled.load(Ordering::SeqCst));
    assert_eq!(*login.calls.lock().unwrap(), vec![true, false]);
    login.refuse_restore.store(true, Ordering::SeqCst);
    assert_eq!(
        store.set(Some(true), Some(false)).unwrap_err(),
        "desktop_preferences_save_failed"
    );
    assert!(store.read().unwrap().launch_at_login);
    assert_eq!(store.close_action(false), CloseAction::Hide);
}
#[test]
fn desktop_preferences_concurrent_partial_updates_are_serialized() {
    let (_root, store, _login) = setup();
    let store = Arc::new(store);
    std::thread::scope(|s| {
        let a = store.clone();
        s.spawn(move || a.set(Some(true), None).unwrap());
        let b = store.clone();
        s.spawn(move || b.set(None, Some(false)).unwrap());
    });
    let view = store.read().unwrap();
    assert!(view.launch_at_login);
    assert!(!view.close_to_tray);
}
#[test]
fn desktop_preferences_close_policy_hides_or_exits() {
    let (_root, store, _login) = setup();
    assert_eq!(store.close_action(false), CloseAction::Hide);
    assert_eq!(store.close_action(true), CloseAction::Exit);
    store.set(None, Some(false)).unwrap();
    assert_eq!(store.close_action(false), CloseAction::Exit);
    assert_eq!(store.close_action(true), CloseAction::Exit);
}

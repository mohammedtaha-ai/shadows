//! Master-key lifecycle with an explicit in-memory credential and temporary locks.
use std::fs::File;
use std::sync::{Arc, mpsc};
use std::time::Duration;

use super::read_or_create_entry;

fn entry() -> Arc<keyring::Entry> {
    Arc::new(keyring::Entry::new_with_credential(Box::new(
        keyring::mock::MockCredential::default(),
    )))
}

#[test]
fn master_key_creation_waits_for_the_other_starter() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("master.lock");
    let held = File::create(&path).unwrap();
    held.lock().unwrap();
    let credential = entry();
    let other = credential.clone();
    let (started, ready) = mpsc::channel();
    let (done, completed) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        started.send(()).unwrap();
        let result = read_or_create_entry(&other, &path);
        done.send(()).unwrap();
        result
    });
    ready.recv().unwrap();
    let ran_early = completed.recv_timeout(Duration::from_millis(150)).is_ok();
    // Simulate the first daemon writing its key while it still owns the lock.
    credential.set_secret(&[7; 32]).unwrap();
    drop(held);
    let result = worker.join().unwrap().unwrap();
    assert!(
        !ran_early,
        "another starter must hold off credential access"
    );
    assert!(result == [7; 32], "both starters use the stored key");
}

#[test]
fn wrong_length_master_key_is_never_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    let credential = entry();
    credential.set_secret(&[7; 31]).unwrap();
    assert!(read_or_create_entry(&credential, &tmp.path().join("master.lock")).is_err());
    assert!(
        credential.get_secret().unwrap() == [7; 31],
        "entry stays intact"
    );
}

#[test]
fn unreadable_master_key_is_never_replaced() {
    let tmp = tempfile::tempdir().unwrap();
    let credential = entry();
    credential.set_secret(&[7; 32]).unwrap();
    let mock: &keyring::mock::MockCredential = credential.get_credential().downcast_ref().unwrap();
    mock.set_error(keyring::Error::Invalid(
        "synthetic".into(),
        "read refused".into(),
    ));
    assert!(read_or_create_entry(&credential, &tmp.path().join("master.lock")).is_err());
    assert!(
        credential.get_secret().unwrap() == [7; 32],
        "entry stays intact"
    );
}

#[test]
fn absent_master_key_is_created_once() {
    let tmp = tempfile::tempdir().unwrap();
    let credential = entry();
    let path = tmp.path().join("master.lock");
    let first = read_or_create_entry(&credential, &path).unwrap();
    let second = read_or_create_entry(&credential, &path).unwrap();
    assert!(first == second, "reopening keeps the same key");
    assert!(
        credential.get_secret().unwrap() == first,
        "the returned key is stored"
    );
}

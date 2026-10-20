use super::send::Submission;
use super::send_fixture::*;
use super::*;
use std::{thread, time::Duration};

#[test]
fn first_send_creates_binds_opens_and_starts_in_order() {
    let (api, _root, p) = fixture(false);
    let scope = p.context().0;
    assert_eq!(p.send(submission(&p, &id())).unwrap().thread_id, T1);
    let calls = api.writes();
    assert_eq!(
        calls.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(),
        ["POST", "POST", "PUT", "POST"]
    );
    assert_eq!(calls[0].1, format!("/api/projects/{PROJECT}/threads"));
    assert_eq!(calls[1].1, format!("/api/threads/{T1}/session"));
    assert_eq!(calls[2].1, format!("/api/threads/{T1}/session/model"));
    assert_eq!(calls[3].1, format!("/api/threads/{T1}/turns"));
    assert_eq!(calls[0].2["profile"], "agent-a");
    assert_eq!(calls[0].2["harness"], "aylul");
    assert_eq!(calls[0].2["title"], "New conversation");
    assert_eq!(calls[3].2["prompt"], "مرحبا 👋\nثاني سطر");
    assert_eq!(calls[3].2["mode"], "ask");
    assert_eq!(calls[3].2["effort"], "medium");
    assert_eq!(calls[3].2["model"], "default-model");
    assert_eq!(p.view().target.unwrap().thread_id, T1);
    assert_eq!(p.context().0, scope);
}
#[test]
fn bound_send_reuses_target_and_keeps_session_settings() {
    let (api, _root, p) = fixture(true);
    p.send(submission(&p, &id())).unwrap();
    let c = api.writes();
    assert_eq!(c.len(), 2);
    assert!(c[0].1.ends_with("/session"));
    assert_eq!(c[1].2["model"], "session-model");
    assert_eq!(c[1].2["mode"], "ask");
    assert_eq!(c[1].2["effort"], "medium");
}
#[test]
fn uncertain_create_retry_makes_one_thread() {
    let (api, _root, p) = fixture(false);
    api.state.lock().unwrap().create_delays = 1;
    let id = id();
    assert_eq!(
        p.send(submission(&p, &id)).unwrap_err(),
        "companion_send_uncertain"
    );
    p.send(submission(&p, &id)).unwrap();
    let c = api.writes();
    assert_eq!(c[0].2["command_id"], c[1].2["command_id"]);
    assert_eq!(api.state.lock().unwrap().turns.len(), 1);
    assert_eq!(p.view().target.unwrap().thread_id, T1);
}
#[test]
fn uncertain_turn_retry_sends_the_captured_request() {
    let (api, _root, p) = fixture(true);
    api.state.lock().unwrap().turn_delays = 1;
    let id = id();
    let original = submission(&p, &id);
    let old_context = original.context.clone();
    assert_eq!(p.send(original).unwrap_err(), "companion_send_uncertain");
    let mut changed = submission(&p, &id);
    changed.text = "changed".into();
    let before = api.writes().len();
    assert_eq!(p.send(changed).unwrap_err(), "companion_send_invalid");
    assert_eq!(api.writes().len(), before);
    p.previews(false).unwrap();
    assert_ne!(p.context().1.unwrap(), old_context);
    api.state.lock().unwrap().settings = true;
    assert_eq!(
        p.send(submission(&p, &id)).unwrap().operation_id,
        "operation-one"
    );
    let c = api.writes();
    assert_eq!(c.len(), 3);
    assert_eq!(c[1].2, c[2].2);
    assert_eq!(api.state.lock().unwrap().turns.len(), 1);
}
#[test]
fn retry_after_main_switches_conversation_is_refused() {
    let (api, _root, p) = fixture(true);
    api.state.lock().unwrap().turn_delays = 1;
    let retry = submission(&p, &id());
    assert_eq!(
        p.send(Submission {
            text: retry.text.clone(),
            command_id: retry.command_id.clone(),
            context: retry.context.clone()
        })
        .unwrap_err(),
        "companion_send_uncertain"
    );
    p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
    let before = api.writes().len();
    assert_eq!(p.send(retry).unwrap_err(), "companion_send_stale");
    assert_eq!(api.writes().len(), before);
    assert!(p.inner.lock().unwrap().send.captured.is_none());
}
#[test]
fn late_old_send_cannot_replace_newer_attempt() {
    let (api, _root, p) = fixture(true);
    let (entered, release) = gate(&api);
    let old_id = id();
    let old = submission(&p, &old_id);
    let owner = p.clone();
    let worker = thread::spawn(move || owner.send(old));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
    api.state.lock().unwrap().turn_delays = 2;
    let new_id = id();
    assert_eq!(
        p.send(submission(&p, &new_id)).unwrap_err(),
        "companion_send_uncertain"
    );
    release.send(()).unwrap();
    assert_eq!(
        worker.join().unwrap().unwrap_err(),
        "companion_send_uncertain"
    );
    let inner = p.inner.lock().unwrap();
    let captured = inner.send.captured.as_ref().unwrap();
    assert_eq!(captured.command_id, new_id);
    assert_eq!(captured.thread_id, T2);
    let calls = api.writes();
    assert!(calls
        .iter()
        .any(|c| c.1 == format!("/api/threads/{T1}/turns") && c.2["command_id"] == old_id));
}
#[test]
fn uncertain_retry_rechecks_runtime_before_writing() {
    let (api, _root, p) = fixture(true);
    api.state.lock().unwrap().turn_delays = 1;
    let command = id();
    assert_eq!(
        p.send(submission(&p, &command)).unwrap_err(),
        "companion_send_uncertain"
    );
    api.state.lock().unwrap().runtime_changed = true;
    let before = api.writes().len();
    assert_eq!(
        p.send(submission(&p, &command)).unwrap_err(),
        "companion_core_unavailable"
    );
    assert_eq!(api.writes().len(), before);
}
#[test]
fn explicit_retry_reconciles_same_configuration_after_verified_runtime_restart() {
    let (api, _root, mut p) = fixture(true);
    api.state.lock().unwrap().turn_delays = 1;
    let command = id();
    assert_eq!(
        p.send(submission(&p, &command)).unwrap_err(),
        "companion_send_uncertain"
    );
    // Replace only native Ready, as after explicit connection Retry has verified
    // the receipt, OS process, database and HTTP identity for this installation.
    Arc::get_mut(&mut p).unwrap().connection = crate::core_connection::Connection::for_binding_test(
        crate::core_connection::ReadyConnection {
            origin: api.origin.clone(),
            configuration_id: CONFIG.into(),
            runtime_id: T1.into(),
        },
    );
    api.state.lock().unwrap().runtime_changed = true;
    api.state.lock().unwrap().settings = true;
    let before = api.writes();
    assert_eq!(
        p.send(submission(&p, &command)).unwrap().operation_id,
        "operation-one"
    );
    let calls = api.writes();
    assert_eq!(calls.len(), before.len() + 1);
    assert_eq!(calls.last(), before.last());
    assert_eq!(api.state.lock().unwrap().turns.len(), 1);
}
#[test]
fn main_switching_conversation_changes_scope() {
    let (_api, _root, p) = fixture(true);
    let before = p.context().0;
    p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
    assert_ne!(p.context().0, before);
}
#[test]
fn stale_context_is_refused_before_any_write() {
    let (api, _root, p) = fixture(true);
    let stale = submission(&p, &id());
    p.select(p.request(None).unwrap(), Some("agent-b".into()))
        .unwrap();
    assert_eq!(p.send(stale).unwrap_err(), "companion_send_stale");
    assert!(api.writes().is_empty());
}
#[test]
fn binding_change_mid_send_completes_on_captured_thread() {
    let (api, _root, p) = fixture(true);
    let (entered, release) = gate(&api);
    let s = submission(&p, &id());
    let owner = p.clone();
    let worker = thread::spawn(move || owner.send(s));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
    release.send(()).unwrap();
    assert_eq!(worker.join().unwrap().unwrap().thread_id, T1);
    assert_eq!(p.view().target.unwrap().thread_id, T2);
}
#[test]
fn first_create_finishes_on_captured_thread_after_agent_or_target_switch() {
    for switch_agent in [true, false] {
        let (api, _root, p) = fixture(false);
        p.inner.lock().unwrap().send.create_timeout = Duration::from_secs(3);
        api.state.lock().unwrap().gate_create = true;
        let (entered, release) = gate(&api);
        let s = submission(&p, &id());
        let owner = p.clone();
        let worker = thread::spawn(move || owner.send(s));
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        if switch_agent {
            p.select(p.request(None).unwrap(), Some("agent-b".into()))
                .unwrap();
        } else {
            p.retarget(p.request(None).unwrap(), T2.into()).unwrap();
        }
        let newer = p.view();
        release.send(()).unwrap();
        assert_eq!(worker.join().unwrap().unwrap().thread_id, T1);
        assert_eq!(p.view(), newer);
        let calls = api.writes();
        assert_eq!(
            calls.iter().filter(|c| c.1.ends_with("/threads")).count(),
            1
        );
        let turn = calls.iter().find(|c| c.1.ends_with("/turns")).unwrap();
        assert_eq!(turn.1, format!("/api/threads/{T1}/turns"));
        assert_eq!(turn.2["model"], "default-model");
    }
}

#[test]
fn first_create_refuses_after_configuration_change() {
    let (api, _root, p) = fixture(false);
    p.inner.lock().unwrap().send.create_timeout = Duration::from_secs(3);
    api.state.lock().unwrap().gate_create = true;
    let (entered, release) = gate(&api);
    let s = submission(&p, &id());
    let owner = p.clone();
    let worker = thread::spawn(move || owner.send(s));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    p.configuration_changed(Some(T2.into()));
    release.send(()).unwrap();
    assert_eq!(worker.join().unwrap().unwrap_err(), "companion_send_stale");
    assert_eq!(api.writes().len(), 1);
}
#[test]
fn detached_first_create_revalidates_runtime_before_session_write() {
    let (api, _root, p) = fixture(false);
    p.inner.lock().unwrap().send.create_timeout = Duration::from_secs(3);
    api.state.lock().unwrap().gate_create = true;
    let (entered, release) = gate(&api);
    let s = submission(&p, &id());
    let owner = p.clone();
    let worker = thread::spawn(move || owner.send(s));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    p.select(p.request(None).unwrap(), Some("agent-b".into()))
        .unwrap();
    api.state.lock().unwrap().runtime_changed = true;
    release.send(()).unwrap();
    assert_eq!(
        worker.join().unwrap().unwrap_err(),
        "companion_core_unavailable"
    );
    assert_eq!(api.writes().len(), 1);
}
#[test]
fn error_codes_map_to_fixed_values() {
    for (code, want) in [
        ("THREAD_BUSY", "companion_send_busy"),
        ("AYLUL_KEY_MISSING", "companion_send_setup"),
        ("MODE_NOT_ALLOWED", "companion_send_settings"),
        ("INTERNAL", "companion_send_failed"),
    ] {
        let (api, _root, p) = fixture(true);
        api.state.lock().unwrap().error = Some(code.into());
        let got = p.send(submission(&p, &id())).unwrap_err();
        assert_eq!(got, want);
        assert!(!got.contains("SECRET"));
    }
}
#[test]
fn removed_target_is_reported_not_replaced() {
    let (api, _root, p) = fixture(true);
    api.state.lock().unwrap().removed = true;
    assert_eq!(
        p.send(submission(&p, &id())).unwrap_err(),
        "companion_thread_invalid"
    );
    assert!(api.writes().is_empty());
    assert_eq!(p.view().target.unwrap().thread_id, T1);
}
#[test]
fn invalid_submissions_send_nothing() {
    let (api, _root, p) = fixture(true);
    for text in ["".to_string(), "   ".into(), "x".repeat(32769)] {
        let mut s = submission(&p, &id());
        s.text = text;
        assert_eq!(p.send(s).unwrap_err(), "companion_send_invalid");
    }
    assert_eq!(
        p.send(submission(&p, "not-uuid")).unwrap_err(),
        "companion_send_invalid"
    );
    assert!(api.state.lock().unwrap().calls.is_empty());
}

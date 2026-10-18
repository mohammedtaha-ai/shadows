use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
};
fn fixture(statuses: Vec<u16>) -> (AuthService, std::thread::JoinHandle<()>, PathBuf) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = url::Url::parse(&format!(
        "http://127.0.0.1:{}",
        listener.local_addr().unwrap().port()
    ))
    .unwrap();
    let path = std::env::temp_dir().join(format!(
        "nylra-auth-policy-{}.dpapi",
        listener.local_addr().unwrap().port()
    ));
    let store = Store::new(path.clone(), base.origin().ascii_serialization());
    let credentials = Credentials {
        handle: Some("h".repeat(43)),
        enrollment: Some("e".repeat(43)),
        account: Some("00000000-0000-4000-8000-000000000001".into()),
        device: Some("00000000-0000-4000-8000-000000000002".into()),
        origin: base.origin().ascii_serialization(),
    };
    store.save(&credentials).unwrap();
    let peer = std::thread::spawn(move || {
        for status in statuses {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut buf = [0; 8192];
            let n = socket.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);
            let (status, body) = if request.starts_with("DELETE ") {
                (200, "{}".to_string())
            } else if status == 200 {
                (200,r#"{"accountId":"00000000-0000-4000-8000-000000000001","deviceId":"00000000-0000-4000-8000-000000000002","sessionId":"00000000-0000-4000-8000-000000000003"}"#.to_string())
            } else {
                (status, "{}".into())
            };
            write!(socket,"HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
    });
    (
        AuthService {
            backend: Backend::fixture(base),
            store,
        },
        peer,
        path,
    )
}
#[test]
fn refusal_and_outage_preserve_enrollment_and_require_live_validation() {
    let (service, peer, path) = fixture(vec![401, 503, 200]);
    assert_eq!(service.status().unwrap().state, "verification_refused");
    let c = service.store.load().unwrap();
    assert!(c.handle.is_some());
    assert!(c.enrollment.is_some());
    assert_eq!(service.status().unwrap().state, "unavailable");
    assert!(service.status().unwrap().principal.is_some());
    peer.join().unwrap();
    crate::native_login::clear_session(&path).unwrap();
}
#[test]
fn current_device_revocation_does_not_require_provider_validation() {
    let (service, peer, path) = fixture(vec![503]);
    assert_eq!(
        service.revoke_current_device().unwrap().state,
        "device_revoked"
    );
    let c = service.store.load().unwrap();
    assert!(c.handle.is_none());
    assert!(c.enrollment.is_none());
    peer.join().unwrap();
    crate::native_login::clear_session(&path).unwrap();
}

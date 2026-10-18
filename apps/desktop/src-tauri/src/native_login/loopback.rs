//! Capture the one loopback login return. The owner's B1 return is IPv4 loopback
//! /oidc-return with a client-generated S256 verifier.
use super::random;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

// Neither type is serializable or Debug: verifier and callback query never enter IPC/logs.
pub struct ReturnBinding {
    listener: TcpListener,
    verifier: String,
}
pub struct NativeReturn {
    pub callback: url::Url,
    verifier: String,
}
impl NativeReturn {
    pub fn verifier(&self) -> &str {
        &self.verifier
    }
}
impl ReturnBinding {
    pub fn new() -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            verifier: URL_SAFE_NO_PAD.encode(random::bytes()?),
        })
    }
    pub fn return_uri(&self) -> String {
        format!(
            "http://127.0.0.1:{}/oidc-return",
            self.listener.local_addr().expect("bound listener").port()
        )
    }
    pub fn challenge(&self) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(self.verifier.as_bytes()))
    }
    pub fn verifier(&self) -> &str {
        &self.verifier
    }
    pub fn wait(self, deadline: Duration) -> io::Result<NativeReturn> {
        self.wait_cancellable(deadline, &std::sync::atomic::AtomicBool::new(false))
    }
    pub fn wait_cancellable(
        self,
        deadline: Duration,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> io::Result<NativeReturn> {
        let start = Instant::now();
        let base = url::Url::parse(&self.return_uri()).expect("fixed native URI");
        while start.elapsed() < deadline {
            if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "login cancelled",
                ));
            }
            let (mut stream, _) = match self.listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(e),
            };
            stream.set_read_timeout(Some(Duration::from_millis(250)))?;
            stream.set_write_timeout(Some(Duration::from_millis(250)))?;
            let mut header = Vec::new();
            let mut chunk = [0; 1024];
            while header.len() < 8192
                && !header.windows(4).any(|w| w == b"\r\n\r\n")
                && start.elapsed() < deadline
                && !cancelled.load(std::sync::atomic::Ordering::Relaxed)
            {
                match stream.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => header.extend_from_slice(&chunk[..n]),
                }
            }
            let callback = if start.elapsed() < deadline
                && !cancelled.load(std::sync::atomic::Ordering::Relaxed)
            {
                std::str::from_utf8(&header)
                    .ok()
                    .and_then(|h| parse_callback(h, &base))
            } else {
                None
            };
            let status = if callback.is_some() {
                "200 OK"
            } else {
                "400 Bad Request"
            };
            let _=stream.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n").as_bytes());
            if let Some(callback) = callback {
                return Ok(NativeReturn {
                    callback,
                    verifier: self.verifier,
                });
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "native return timed out",
        ))
    }
}
fn parse_callback(header: &str, base: &url::Url) -> Option<url::Url> {
    if header.len() >= 8192 || !header.ends_with("\r\n\r\n") {
        return None;
    }
    let mut lines = header.split("\r\n");
    let parts: Vec<_> = lines.next()?.split(' ').collect();
    if parts.len() != 3
        || parts[0] != "GET"
        || parts[2] != "HTTP/1.1"
        || !parts[1].starts_with("/oidc-return?")
        || parts[1].contains('#')
        || parts[1].contains('\\')
    {
        return None;
    }
    let hosts: Vec<_> = lines
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| k.eq_ignore_ascii_case("host"))
        .map(|(_, v)| v.trim())
        .collect();
    if hosts != [format!("127.0.0.1:{}", base.port()?).as_str()] {
        return None;
    }
    let u = base.join(parts[1]).ok()?;
    if u.origin() != base.origin()
        || u.path() != "/oidc-return"
        || u.query().is_none_or(str::is_empty)
    {
        return None;
    }
    Some(u)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_binding_is_random_and_s256() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use sha2::{Digest, Sha256};
        let a = ReturnBinding::new().unwrap();
        let b = ReturnBinding::new().unwrap();
        assert_eq!(a.verifier().len(), 43);
        assert_ne!(a.verifier(), b.verifier());
        assert_eq!(
            a.challenge(),
            URL_SAFE_NO_PAD.encode(Sha256::digest(a.verifier().as_bytes()))
        );
        let u = url::Url::parse(&a.return_uri()).unwrap();
        assert_eq!(u.host_str(), Some("127.0.0.1"));
        assert_eq!(u.path(), "/oidc-return");
        assert!(u.port().unwrap() > 0);
    }
    #[test]
    fn loopback_ignores_foreign_host_and_returns_one_owned_callback() {
        use std::{
            io::{Read, Write},
            net::TcpStream,
            time::Duration,
        };
        let b = ReturnBinding::new().unwrap();
        let uri = b.return_uri();
        let u = url::Url::parse(&uri).unwrap();
        let port = u.port().unwrap();
        let peer = std::thread::spawn(move || {
            for host in [format!("evil.example:{port}"), format!("127.0.0.1:{port}")] {
                let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
                stream
                    .write_all(
                        format!("GET /oidc-return?ticket=fixture HTTP/1.1\r\nHost: {host}\r\n\r\n")
                            .as_bytes(),
                    )
                    .unwrap();
                let mut response = String::new();
                stream.read_to_string(&mut response).unwrap();
                assert!(response.contains(if host.starts_with("evil") {
                    "400"
                } else {
                    "200"
                }));
            }
        });
        let returned = b.wait(Duration::from_secs(5)).unwrap();
        assert_eq!(returned.callback.query(), Some("ticket=fixture"));
        peer.join().unwrap();
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
    }
    #[test]
    fn cancellation_closes_native_listener() {
        let b = ReturnBinding::new().unwrap();
        let u = url::Url::parse(&b.return_uri()).unwrap();
        let result = b.wait_cancellable(
            Duration::from_secs(1),
            &std::sync::atomic::AtomicBool::new(true),
        );
        assert!(matches!(result,Err(e)if e.kind()==io::ErrorKind::Interrupted));
        assert!(std::net::TcpStream::connect(("127.0.0.1", u.port().unwrap())).is_err());
    }
    #[test]
    fn callback_rejects_wrong_method_path_fragment_and_duplicate_host() {
        let base = url::Url::parse("http://127.0.0.1:12345/oidc-return").unwrap();
        for header in ["POST /oidc-return?x=1 HTTP/1.1\r\nHost: 127.0.0.1:12345\r\n\r\n","GET /other?x=1 HTTP/1.1\r\nHost: 127.0.0.1:12345\r\n\r\n","GET /oidc-return?x=1#f HTTP/1.1\r\nHost: 127.0.0.1:12345\r\n\r\n","GET /oidc-return?x=1 HTTP/1.1\r\nHost: 127.0.0.1:12345\r\nHost: 127.0.0.1:12345\r\n\r\n"] {assert!(parse_callback(header,&base).is_none());}
    }
}

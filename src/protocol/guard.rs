//! One job: refusing a request that a browser sent on behalf of a page that
//! is not one of this daemon's clients.
//!
//! CORS alone does not do this. It decides whether a page may *read* an
//! answer, not whether the request runs: a page on any site can make a
//! browser send a simple request — a `GET` through an `<img>`, a body-less
//! `POST` from a form — and the handler runs before the browser withholds the
//! answer. Here that would let any web page make the daemon list a path of its
//! choosing (a `\\host\share` path makes Windows authenticate to that host)
//! or stop a turn. And DNS rebinding goes around CORS entirely: a page whose
//! own name is re-pointed at 127.0.0.1 is same-origin with the daemon, so the
//! browser lets it read everything and send anything.
//!
//! So two checks run before any route, both on headers a page cannot forge:
//!
//! - **`Host`** must be `localhost` (or a name under it) or an IP address. A
//!   rebinding attack needs the attacker's own name in `Host`; an IP address
//!   cannot be rebound. A request without `Host` is not from a browser.
//! - **`Origin`**, when present, must be one of the allowed origins. When it
//!   is absent but the browser says the request came from another site
//!   (`Sec-Fetch-Site`), it is a no-cors request from a page, and is refused.
//!   A request with neither — `curl`, a test, a tab opened on a URL — passes.
//!
//! A preflight never reaches this: the CORS layer outside it answers those.

use std::net::IpAddr;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::{AppState, Failure};

pub(super) async fn refuse_foreign_pages(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    match foreign(request.headers(), &state.allowed_origins) {
        Some(why) => {
            tracing::warn!(why, "http.refused");
            Failure::origin_refused(why).into_response()
        }
        None => next.run(request).await,
    }
}

/// Why this request is refused, or `None` to let it through.
fn foreign(headers: &HeaderMap, allowed: &[String]) -> Option<&'static str> {
    if let Some(host) = headers.get(header::HOST)
        && !host.to_str().is_ok_and(local_or_literal)
    {
        return Some("the Host header names neither this machine nor an IP address");
    }
    match headers.get(header::ORIGIN) {
        Some(origin) if !allowed.iter().any(|a| a.as_bytes() == origin.as_bytes()) => {
            Some("the request's origin is not an allowed origin")
        }
        Some(_) => None,
        None => match headers.get("sec-fetch-site").map(|v| v.as_bytes()) {
            Some(b"same-origin" | b"none") | None => None,
            Some(_) => Some("a page on another site sent this request without an origin"),
        },
    }
}

/// `host` (with or without a port) is `localhost`, a name under it, or an IP
/// address literal.
fn local_or_literal(host: &str) -> bool {
    let name = match host.strip_prefix('[') {
        // `[v6]` or `[v6]:port`.
        Some(rest) => match rest.split_once(']') {
            Some((v6, port)) if port.is_empty() || port.starts_with(':') => v6,
            _ => return false,
        },
        None => match host.rsplit_once(':') {
            Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => name,
            Some(_) => return false,
            None => host,
        },
    };
    let name = name.to_ascii_lowercase();
    name == "localhost" || name.ends_with(".localhost") || name.parse::<IpAddr>().is_ok()
}

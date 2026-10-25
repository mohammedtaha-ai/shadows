use std::collections::HashMap;
use tauri::{
    utils::config::{Csp, CspDirectiveSources},
    Manager, WebviewUrl, WebviewWindowBuilder,
};

pub fn build(app: &tauri::App) -> Result<tauri::WebviewWindow, tauri::Error> {
    let connection = app
        .state::<std::sync::Arc<crate::app::DesktopApp>>()
        .connection()
        .clone();
    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("AYLUL")
        .inner_size(1200., 800.)
        .min_inner_size(640., 400.)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_navigation(|url| {
            (matches!(url.scheme(), "tauri")
                || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost")))
                && matches!(url.path(), "/" | "/index.html")
        })
        .on_web_resource_request(move |request, response| {
            reject_html_fallback(request.uri().path(), "/index.html", response);
            if let Some(header) = response.headers_mut().get_mut("Content-Security-Policy") {
                if let Ok(value) = header.to_str() {
                    let mut policy: HashMap<String, CspDirectiveSources> =
                        Csp::Policy(value.to_owned()).into();
                    if let Some(config) = connection.configured() {
                        policy
                            .entry("connect-src".into())
                            .or_default()
                            .push(config.origin);
                    }
                    policy
                        .entry("style-src-attr".into())
                        .or_default()
                        .push("'unsafe-inline'");
                    if let Ok(value) = Csp::from(policy).to_string().parse() {
                        *header = value;
                    }
                }
            }
        });
    if let Some(directory) = app
        .state::<std::sync::Arc<crate::app::DesktopApp>>()
        .paths()
        .webview("main")
    {
        builder = builder.data_directory(directory);
    }
    if let Some(args) = app
        .state::<std::sync::Arc<crate::app::DesktopApp>>()
        .paths()
        .browser_args("main")
    {
        builder = builder.additional_browser_args(&args);
    }
    builder.build()
}

/// Tauri's SPA fallback must not return main HTML for missing script/assets.
pub fn reject_html_fallback(
    path: &str,
    entry: &str,
    response: &mut tauri::http::Response<std::borrow::Cow<'static, [u8]>>,
) {
    let html = response
        .headers()
        .get(tauri::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    if html && path != entry && !(entry == "/index.html" && path == "/") {
        *response.status_mut() = tauri::http::StatusCode::NOT_FOUND;
        response.headers_mut().insert(
            tauri::http::header::CONTENT_TYPE,
            tauri::http::HeaderValue::from_static("text/plain"),
        );
        *response.body_mut() = std::borrow::Cow::Borrowed(b"Asset not found");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_asset_is_not_html_but_entry_remains_html() {
        for (path, status) in [("/missing.js", 404), ("/index.html", 200), ("/", 200)] {
            let mut response = tauri::http::Response::builder()
                .header("content-type", "text/html")
                .body(std::borrow::Cow::Borrowed(&b"<html>main</html>"[..]))
                .unwrap();
            reject_html_fallback(path, "/index.html", &mut response);
            assert_eq!(response.status().as_u16(), status);
        }
    }
}

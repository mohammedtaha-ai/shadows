//! Opt-in native library integration, synthetic Compose account only. No UI claim.
use super::service::AuthService;
use reqwest::{blocking::Client, redirect::Policy};
use std::{collections::BTreeMap, path::PathBuf, sync::atomic::AtomicBool, time::Duration};

fn driver(start: &url::Url, base: &url::Url) -> Result<(), &'static str> {
    let root =
        PathBuf::from(std::env::var_os("NYLRA_ACCEPTANCE_RUNTIME").ok_or("test_runtime_missing")?);
    let ca = std::fs::read(root.join("tls/ca.pem")).map_err(|_| "test_config_invalid")?;
    let password = std::fs::read_to_string(root.join("secrets/user-password"))
        .map_err(|_| "test_config_invalid")?;
    let client = Client::builder()
        .no_proxy()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(15))
        .add_root_certificate(
            reqwest::Certificate::from_pem(&ca).map_err(|_| "test_config_invalid")?,
        )
        .build()
        .map_err(|_| "test_config_invalid")?;
    let mut cookies = BTreeMap::new();
    let mut current = start.clone();
    for _ in 0..20 {
        if current.scheme() == "http"
            && current.host_str() == Some("127.0.0.1")
            && current.path() == "/oidc-return"
        {
            let target = current.clone();
            // Native wait begins after this driver returns; a real TCP request is queued.
            std::thread::spawn(move || {
                let _ = client.get(target).send();
            });
            return Ok(());
        }
        if current.origin() != base.origin() {
            return Err("test_redirect_invalid");
        }
        let cookie = cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        let response = client
            .get(current.clone())
            .header("cookie", cookie)
            .send()
            .map_err(|_| "test_request_failed")?;
        for c in response.headers().get_all("set-cookie") {
            if let Ok(c) = c.to_str() {
                if let Some((k, v)) = c.split(';').next().unwrap_or("").split_once('=') {
                    cookies.insert(k.to_string(), v.to_string());
                }
            }
        }
        if let Some(location) = response.headers().get("location") {
            current = current
                .join(location.to_str().map_err(|_| "test_redirect_invalid")?)
                .map_err(|_| "test_redirect_invalid")?;
            continue;
        }
        if current.path() == "/v1/auth/logout-callback" && response.status().is_success() {
            return Ok(());
        }
        let body = response.text().map_err(|_| "test_request_failed")?;
        let form = body
            .split("id=\"kc-form-login\"")
            .nth(1)
            .and_then(|s| s.split("action=\"").nth(1))
            .and_then(|s| s.split('"').next())
            .ok_or("test_login_form_missing")?;
        let action = current
            .join(&form.replace("&amp;", "&"))
            .map_err(|_| "test_redirect_invalid")?;
        if action.origin() != base.origin() {
            return Err("test_redirect_invalid");
        }
        let cookie = cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        let response = client
            .post(action)
            .header("cookie", cookie)
            .form(&[
                ("username", "synthetic-user"),
                ("password", password.trim()),
                ("credentialId", ""),
            ])
            .send()
            .map_err(|_| "test_request_failed")?;
        for c in response.headers().get_all("set-cookie") {
            if let Ok(c) = c.to_str() {
                if let Some((k, v)) = c.split(';').next().unwrap_or("").split_once('=') {
                    cookies.insert(k.to_string(), v.to_string());
                }
            }
        }
        current = current
            .join(
                response
                    .headers()
                    .get("location")
                    .ok_or("test_redirect_invalid")?
                    .to_str()
                    .map_err(|_| "test_redirect_invalid")?,
            )
            .map_err(|_| "test_redirect_invalid")?;
    }
    Err("test_redirect_bound")
}

#[test]
#[ignore = "requires authorized local Docker synthetic identity runtime"]
fn native_compose_login_reauthentication_revocation_and_logout() {
    let root =
        PathBuf::from(std::env::var_os("NYLRA_ACCEPTANCE_RUNTIME").expect("explicit test runtime"));
    let path = root.join(format!("native-acceptance-{}.dpapi", std::process::id()));
    let service = AuthService::from_environment(path.clone()).unwrap();
    let stop = AtomicBool::new(false);
    let first = service.login_with_browser(true, &stop, driver).unwrap();
    assert_eq!(first.state, "signed_in");
    let principal = first.principal.unwrap();
    let restarted = AuthService::from_environment(path.clone()).unwrap();
    let child =
        std::env::var_os("NYLRA_ACCEPTANCE_CHILD").expect("explicit acceptance child executable");
    let output = std::process::Command::new(child)
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let child_state: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(child_state["state"], "signed_in");
    assert_eq!(child_state["deviceId"], principal.device_id);
    assert_eq!(
        restarted.status().unwrap().principal.unwrap().device_id,
        principal.device_id
    );
    let second = service
        .login_with_browser(false, &stop, driver)
        .unwrap()
        .principal
        .unwrap();
    assert_eq!(second.device_id, principal.device_id);
    assert_ne!(second.session_id, principal.session_id);
    let projection = serde_json::to_string(&service.status().unwrap()).unwrap();
    assert!(!projection.contains("handle"));
    assert!(!projection.contains("credential"));
    assert_eq!(
        service.revoke_current_device().unwrap().state,
        "device_revoked"
    );
    assert_eq!(restarted.status().unwrap().state, "signed_out");
    assert!(service.login_with_browser(false, &stop, driver).is_err());
    let explicit = service.login_with_browser(true, &stop, driver).unwrap();
    assert_ne!(explicit.principal.unwrap().device_id, principal.device_id);
    let out = service.logout_with_browser(false, driver).unwrap();
    assert_eq!(out.state, "signed_out");
    assert!(out.has_enrollment);
    assert_eq!(out.provider_logout, Some("pending"));
    let final_state = AuthService::from_environment(path.clone())
        .unwrap()
        .status()
        .unwrap();
    assert_eq!(final_state.state, "signed_out");
    assert!(final_state.has_enrollment);
    crate::native_login::clear_session(&path).unwrap();
}

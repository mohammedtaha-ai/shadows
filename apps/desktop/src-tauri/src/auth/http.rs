use super::wire;
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{io::Read, time::Duration};

pub struct Backend {
    pub base: url::Url,
    client: Client,
}
impl Backend {
    #[cfg(test)]
    pub(super) fn fixture(base: url::Url) -> Self {
        assert_eq!(base.host_str(), Some("127.0.0.1"));
        Self {
            base,
            client: Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap(),
        }
    }
    pub fn from_environment() -> Result<Self, &'static str> {
        let base =
            std::env::var("NYLRA_BACKEND_URL").unwrap_or_else(|_| "https://127.0.0.1:8443".into());
        let base = url::Url::parse(&base).map_err(|_| "configuration_invalid")?;
        if base.scheme() != "https"
            || !base.username().is_empty()
            || base.password().is_some()
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err("configuration_invalid");
        }
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .https_only(true);
        if base.host_str() == Some("127.0.0.1") {
            builder = builder.no_proxy();
        }
        if let Some(path) = std::env::var_os("NYLRA_DEV_CA") {
            if base.host_str() != Some("127.0.0.1") {
                return Err("configuration_invalid");
            }
            let pem = std::fs::read(path).map_err(|_| "certificate_unavailable")?;
            let certificate =
                reqwest::Certificate::from_pem(&pem).map_err(|_| "certificate_invalid")?;
            builder = builder.add_root_certificate(certificate);
        }
        Ok(Self {
            base,
            client: builder.build().map_err(|_| "transport_unavailable")?,
        })
    }
    pub fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
        handle: Option<&str>,
    ) -> Result<T, &'static str> {
        let url = self.base.join(path).map_err(|_| "configuration_invalid")?;
        if url.origin() != self.base.origin() {
            return Err("configuration_invalid");
        }
        let mut request = self.client.request(method, url);
        if let Some(value) = body {
            request = request.json(&value);
        }
        if let Some(handle) = handle {
            if !wire::opaque(handle) {
                return Err("credential_invalid");
            }
            request = request.bearer_auth(handle);
        }
        let response = request.send().map_err(|_| "backend_unavailable")?;
        let status = response.status();
        // Refusals never become proof of enrollment revocation. Raw errors/bodies remain private.
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 => "verification_refused",
                400 => "request_refused",
                429 => "rate_limited",
                _ => "backend_unavailable",
            });
        }
        let mut bytes = Vec::new();
        response
            .take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| "backend_unavailable")?;
        if bytes.len() > 16384 {
            return Err("response_invalid");
        }
        serde_json::from_slice(&bytes).map_err(|_| "response_invalid")
    }
}

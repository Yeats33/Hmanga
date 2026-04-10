use std::collections::HashMap;

use hmanga_core::{HostApi, HttpMethod, HttpRequest, HttpResponse, LogLevel, PluginError};
use reqwest::Client;

/// HostRuntime is the common HTTP-capable runtime exposed to site plugins.
#[derive(Debug, Clone)]
pub struct HostRuntime {
    client: Client,
}

impl HostRuntime {
    pub fn new() -> Self {
        Self::new_with_proxy(None).expect("failed to build reqwest client")
    }

    pub fn new_with_proxy(proxy: Option<&str>) -> Result<Self, String> {
        let mut builder = Client::builder().cookie_store(true);
        if let Some(proxy) = proxy.filter(|value| !value.trim().is_empty()) {
            builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|err| err.to_string())?);
        }
        let client = builder.build().map_err(|err| err.to_string())?;
        Ok(Self { client })
    }
}

impl Default for HostRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl HostApi for HostRuntime {
    fn http_request(
        &self,
        request: HttpRequest,
    ) -> impl std::future::Future<Output = Result<HttpResponse, PluginError>> + Send {
        let client = self.client.clone();
        async move {
            let method = match request.method {
                HttpMethod::Get => reqwest::Method::GET,
                HttpMethod::Post => reqwest::Method::POST,
                HttpMethod::Put => reqwest::Method::PUT,
                HttpMethod::Delete => reqwest::Method::DELETE,
            };

            let mut builder = client.request(method, &request.url);
            for (key, value) in &request.headers {
                builder = builder.header(key, value);
            }
            if let Some(body) = request.body {
                builder = builder.body(body);
            }

            let response = builder
                .send()
                .await
                .map_err(|err| PluginError::Network(err.to_string()))?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect::<HashMap<_, _>>();
            let body = response
                .bytes()
                .await
                .map_err(|err| PluginError::Network(err.to_string()))?
                .to_vec();

            Ok(HttpResponse {
                status,
                headers,
                body,
            })
        }
    }

    fn log(&self, _level: LogLevel, _message: &str) {}
}

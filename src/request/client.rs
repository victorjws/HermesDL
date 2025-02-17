use super::constant::TOR_PROXY_SCHEME;
use super::response::Response;
use super::user_agent::UserAgent;
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client as ReqwestClient, Proxy};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Client {
    inner: ReqwestClient,
    default_headers: HashMap<String, String>,
}

impl Client {
    pub fn new(use_tor: bool, user_agent: &UserAgent) -> Result<Self> {
        let mut client_builder = ReqwestClient::builder().user_agent(user_agent.clone());
        if use_tor {
            let proxy = Proxy::all(TOR_PROXY_SCHEME)?;
            client_builder = client_builder.proxy(proxy);
        }
        let client = client_builder.build()?;
        Ok(Self {
            inner: client,
            default_headers: HashMap::new(),
        })
    }

    pub async fn head(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<Response> {
        let headers = self.convert_headers(headers);
        let response = self.inner.head(url).headers(headers).send().await?;
        Ok(Response::new(response))
    }

    pub async fn get(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<Response> {
        let headers = self.convert_headers(headers);
        let response = self.inner.get(url).headers(headers).send().await?;
        Ok(Response::new(response))
    }

    fn convert_headers(&self, extra_headers: Option<&HashMap<String, String>>) -> HeaderMap {
        let mut header_map = HeaderMap::new();

        for (key, value) in &self.default_headers {
            if let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = HeaderValue::from_str(value) {
                    header_map.insert(header_name, header_value);
                }
            }
        }

        if let Some(extra) = extra_headers {
            for (key, value) in extra {
                if let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(header_value) = HeaderValue::from_str(value) {
                        header_map.insert(header_name, header_value);
                    }
                }
            }
        }

        header_map
    }
}

impl fmt::Display for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Client()")
    }
}

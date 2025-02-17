use anyhow::Result;
use bytes::Bytes;
use futures::TryStreamExt;
use futures_core::Stream;
use reqwest::header::{
    HeaderName, ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE,
};
use reqwest::Response as ReqwestResponse;

#[derive(Debug)]
pub struct Response {
    inner: ReqwestResponse,
}

impl Response {
    pub fn new(response: ReqwestResponse) -> Self {
        Self { inner: response }
    }

    fn get_from_header(&self, header_name: HeaderName) -> Option<String> {
        if let Some(value) = self.inner.headers().get(header_name) {
            if let Ok(content) = value.to_str() {
                return Some(content.to_string());
            }
        }
        None
    }

    pub fn accept_ranges(&self) -> Option<String> {
        self.get_from_header(ACCEPT_RANGES)
    }

    pub fn content_disposition(&self) -> Option<String> {
        self.get_from_header(CONTENT_DISPOSITION)
    }

    pub fn content_type(&self) -> Option<String> {
        self.get_from_header(CONTENT_TYPE)
    }

    pub fn content_length(&self) -> Option<u64> {
        if let Some(size) = self.get_from_header(CONTENT_LENGTH) {
            if let Ok(size) = size.parse::<u64>() {
                return Some(size);
            }
        }
        None
    }

    pub fn bytes_stream(self) -> impl Stream<Item = Result<Bytes>> + Unpin {
        self.inner.bytes_stream().map_err(anyhow::Error::from)
    }

    pub async fn text(self) -> Option<String> {
        match self.inner.text().await {
            Ok(text) => Some(text),
            Err(_) => None,
        }
    }
}

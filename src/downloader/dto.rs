use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct DownloadInfo {
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
}

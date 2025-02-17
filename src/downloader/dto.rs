use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DownloadInfo {
    pub url: String,
}
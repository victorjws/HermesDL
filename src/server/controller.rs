use crate::downloader::dto::DownloadInfo;
use crate::downloader::manager;
use crate::server::config::{Config, SharedConfig};
use anyhow::Result;
use std::collections::HashMap;
use std::convert::Infallible;
use warp::{Filter, Reply};

pub fn with_shared_config(
    shared_config: SharedConfig,
) -> impl Filter<Extract = (SharedConfig,), Error = Infallible> + Clone {
    warp::any().map(move || shared_config.clone())
}

fn modify_header(header: &mut HashMap<String, String>) {
    let keys = [
        "Cache-Control",
        "Pragma",
        "If-Modified-Since",
        "If-None-Match",
        "User-Agent",
    ];
    for key in keys {
        header.remove(&key.to_lowercase());
    }
}

pub async fn init_download(
    mut info: DownloadInfo,
    shared_config: SharedConfig,
) -> Result<impl Reply, Infallible> {
    let config = shared_config.read().await;
    let downloader = manager::Downloader::new(
        config.use_tor,
        &config.user_agent,
        config.chunk_size,
        config.max_concurrent_count,
    );

    if let Some(ref mut headers) = info.headers {
        modify_header(headers);
    };

    if let Err(e) = downloader
        .download_file(info.url.as_str(), info.headers.as_ref())
        .await
    {
        eprintln!("{e}");
    }
    Ok("success")
}

pub async fn update_config(
    new_config: Config,
    shared_config: SharedConfig,
) -> Result<impl Reply, Infallible> {
    if let Err(e) = Config::update(new_config, shared_config).await {
        eprintln!("{e}");
    };
    Ok("success")
}

use crate::downloader::dto::DownloadInfo;
use crate::downloader::manager;
use crate::server::config::{Config, SharedConfig};
use anyhow::Result;
use std::convert::Infallible;
use warp::{Filter, Reply};

pub fn with_shared_config(
    shared_config: SharedConfig,
) -> impl Filter<Extract = (SharedConfig,), Error = Infallible> + Clone {
    warp::any().map(move || shared_config.clone())
}

pub async fn init_download(
    info: DownloadInfo,
    shared_config: SharedConfig,
) -> Result<impl Reply, Infallible> {
    let config = shared_config.read().await;
    let downloader = manager::Downloader::new(
        config.use_tor,
        &config.user_agent,
        config.chunk_size,
        config.max_concurrent_count,
    );

    if let Err(e) = downloader.download_file(info.url.as_str()).await {
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

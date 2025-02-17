use super::config::create_shared_config;
use super::constant;
use super::controller::{init_download, update_config, with_shared_config};
use warp::Filter;

pub async fn run_server() {
    let shared_config = create_shared_config().await;

    let download_route = warp::post()
        .and(warp::path("download"))
        .and(warp::body::json())
        .and(with_shared_config(shared_config.clone()))
        .and_then(init_download);

    let update_config_route = warp::put()
        .and(warp::path("config"))
        .and(warp::body::json())
        .and(with_shared_config(shared_config.clone()))
        .and_then(update_config);

    let routes = download_route.or(update_config_route);

    println!("Start Server");
    warp::serve(routes)
        .run((constant::SERVER_ADDRESS, constant::SERVER_PORT))
        .await;
}

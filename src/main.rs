use crate::server::runner::run_server;

mod downloader;
mod request;
mod server;

#[tokio::main]
async fn main() {
    run_server().await;
}

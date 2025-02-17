use super::constant;
use crate::request::user_agent::UserAgent;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub use_tor: bool,
    pub user_agent: UserAgent,
    pub chunk_size: u64,
    pub max_concurrent_count: usize,
}

pub type SharedConfig = Arc<RwLock<Config>>;

impl Config {
    pub fn new() -> Self {
        Self {
            use_tor: false,
            user_agent: UserAgent::Chrome,
            chunk_size: 10_000_000,
            max_concurrent_count: 5,
        }
    }

    pub async fn load() -> Self {
        match Self::load_from_file().await {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Load Failed: {e}");
                let config = Self::new();
                if let Err(e) = config.save().await {
                    eprintln!("Save Failed: {e}");
                };
                config
            }
        }
    }

    pub async fn update(new_config: Config, shared_config: SharedConfig) -> Result<()> {
        let mut config = shared_config.write().await;
        *config = new_config.clone();

        config.save().await?;

        Ok(())
    }

    async fn load_from_file() -> Result<Self> {
        let data = fs::read_to_string(constant::CONFIG_PATH).await?;
        let config: Config = serde_json::from_str(&data)?;
        Ok(config)
    }

    async fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self)?;
        fs::write(constant::CONFIG_PATH, data).await?;
        Ok(())
    }
}

pub async fn create_shared_config() -> SharedConfig {
    Arc::new(RwLock::new(Config::load().await))
}

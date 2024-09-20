use anyhow::{Context, Error};
use config::Config;

#[derive(Debug, serde_derive::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub docker_socket: String,
    pub docker_network: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            docker_socket: "/var/run/docker.sock".to_string(),
            docker_network: "rokku".to_string(),
        }
    }
}

pub fn load_config() -> Result<AppConfig, Error> {
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("rokku"))
        .build()
        .context("Can't load configuration")?;
    
    config
        .try_deserialize()
        .context("Can't deserialize AppConfig from loaded configuration")
}

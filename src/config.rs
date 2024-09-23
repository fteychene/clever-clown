use anyhow::{Context, Error};
use config::Config;

#[derive(Debug, Clone, serde_derive::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub docker: DockerConfig,
    pub api: ApiConfig,
    pub sourcedirectory: String, // TODO move to a path to check exists and avoid trailing slashes
}

#[derive(Debug, Clone, serde_derive::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DockerConfig {
    pub socket: String,
    pub network: String,
}

#[derive(Debug, Clone, serde_derive::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ApiConfig {
    pub host: String,
    pub port: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            docker: Default::default(),
            api: Default::default(),
            sourcedirectory: "/tmp".to_string(),
        }
    }
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            socket: "/var/run/docker.sock".to_string(),
            network: "cleverclown".to_string(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000.to_string(),
        }
    }
}

pub fn load_config() -> Result<AppConfig, Error> {
    let config = Config::builder()
        .add_source(config::Environment::with_prefix("cleverclown").separator("_"))
        .build()
        .context("Can't load configuration")?;

    config
        .try_deserialize()
        .context("Can't deserialize AppConfig from loaded configuration")
}

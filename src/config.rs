use anyhow::{Context, Error};
use config::Config;
use serde_derive::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub docker: DockerConfig,
    pub api: ApiConfig,
    pub sourcedirectory: String, // TODO move to a path to check exists and avoid trailing slashes
    pub routing: RoutingConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DockerConfig {
    pub socket: String,
    pub network: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ApiConfig {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RoutingConfig {
    pub domain: String, // TODO check domain is http acceptable domain
    pub dashboard: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            docker: Default::default(),
            api: Default::default(),
            sourcedirectory: "/tmp".to_string(),
            routing: Default::default(),
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

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            domain: "clever.clown".to_string(), // TODO decide extension cause clown is not a usable TLD 
            dashboard: true,
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

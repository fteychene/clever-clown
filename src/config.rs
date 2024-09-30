use anyhow::{Context, Error};
use config::Config;
use log::LevelFilter;
use serde_derive::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub orchestrator: Orchestrator,
    pub api: ApiConfig,
    pub routing: RoutingConfig,
    #[serde(rename(deserialize = "loglevel"))]
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DockerConfig {
    pub socket: String,
    pub network: String,
    #[serde(rename(deserialize = "sourcedirectory"))]
    pub source_directory: String, // TODO move to a path to check exists and avoid trailing slashes
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct KubernetesConfig {
    #[serde(rename(deserialize = "appnamespace"))]
    pub app_namespace: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum Orchestrator {
    Docker(DockerConfig),
    Kubernetes(KubernetesConfig)
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            orchestrator: Orchestrator::Docker(Default::default()),
            api: Default::default(),
            routing: Default::default(),
            log_level: LevelFilter::Info.to_string(),
        }
    }
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            socket: "/var/run/docker.sock".to_string(),
            network: "cleverclown".to_string(),
            source_directory: "/tmp".to_string(),
        }
    }
}

impl Default for KubernetesConfig {
    fn default() -> Self {
        Self { app_namespace: "default".to_string() }
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

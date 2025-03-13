use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Application {
    pub name: String,
    pub source: ApplicationSource,
    pub configuration: Option<ApplicationConfig>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub domain: Option<String>,
    pub exposed_port: Option<u16>,
    pub replicas: Option<u8>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ApplicationSource {
    DockerImage {
        image: String,
        pull: bool,
        // TODO repository and credentials
    },
    Git {
        remote: String,
        dockerfile: Option<String>,
        // TODO credentials
    },
    LocalRepo {
        path: String,
        dockerfile: Option<String>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub started_at: u64,
    pub image_id: String,
    pub labels: HashMap<String, String>,
}

#[derive(Clone, Serialize)]
pub struct RunningApplication {
    pub name: String,
    pub domain: String,
    pub containers: Vec<Container>
}

impl ApplicationConfig {
    pub fn configuration_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        let conf = self.env.clone().unwrap_or_else(|| HashMap::new());
        let generated_conf = conf
            .into_iter()
            .map(|(key, val)| format!("{}={}", key, val))
            .collect_vec();
        generated_conf.hash(&mut hasher);
        hasher.finish()
    }
}

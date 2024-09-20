use serde::{Deserialize, Serialize};


#[derive(Clone, Serialize, Deserialize)]
pub struct Application {
    pub name: String,
    pub source: ApplicationSource,
    pub domain: Option<String>,
    pub exposed_port: Option<u16>,
    pub replicas: Option<u8>,
    // pub created_at: Instant,
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
        // TODO credentials
    },
    LocalRepo {
        path: String
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub started_at: u64,
    pub image_id: String,
}

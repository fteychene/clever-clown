use std::error::Error;

use anyhow::Context;
use bollard::{Docker, API_DEFAULT_VERSION};
use config::load_config;
use infra::{docker::DockerContainerExecutor, web::router};
use log::{debug, info};
use tokio::net::TcpListener;

mod config;
mod domain;
mod infra;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    info!("Start CleverClown - Your Rust single instance PaaS for learning purpose");

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let config = load_config()?;
    info!("Loaded config {:?}", config);
    let http_bind = format!("{}:{}", config.http.host, config.http.port);

    let docker = Docker::connect_with_socket(&config.docker.socket, 120, API_DEFAULT_VERSION)
        .context("Can't connect to docker socket")?;
    debug!("Docker version : {:?}", docker.version().await?.version);

    let service = domain::ReconciliationService {
        container_executor: Box::new(DockerContainerExecutor {
            config: config,
            docker,
        }),
    };

    info!("Start cleverclown http server on {}", http_bind);
    let listener = TcpListener::bind(http_bind).await.unwrap();
    axum::serve(listener, router(service)).await?;
    Ok(())
}

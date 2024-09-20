use std::error::Error;

use anyhow::Context;
use bollard::{Docker, API_DEFAULT_VERSION};
use config::load_config;
use infra::{docker::DockerContainerExecutor, web::router};
use tokio::net::TcpListener;

mod config;
mod domain;
mod infra;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Start Rokku - Your Rust single instance PaaS for learning purpose");
    
    let config = load_config()?;
    println!("Loaded config {:?}", config);
    let docker = Docker::connect_with_socket(&config.docker_socket, 120, API_DEFAULT_VERSION).context("Can't connect to docker socket")?;
    // println!("Docker version : {:?}", docker.version().await?.version);

    let service = domain::ReconciliationService {
        container_executor: Box::new(DockerContainerExecutor { config, docker }),
    };

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router(service)).await?;
    Ok(())
}


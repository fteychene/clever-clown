use std::{error::Error, str::FromStr};

use anyhow::Context;
use bollard::{Docker, API_DEFAULT_VERSION};
use config::{load_config, Orchestrator};
use domain::port::ContainerExecutor;
use infra::{
    docker::DockerContainerExecutor, kubernetes::KubernetesContainerExecutor, web::router,
};
use kube::Client;
use log::{info, warn, LevelFilter};
use tokio::net::TcpListener;

mod config;
mod domain;
mod infra;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    info!("Start CleverClown - Your Rust PaaS for learning purpose");
    let config = load_config()?;

    env_logger::builder()
        .filter_level(
            FromStr::from_str(config.log_level.as_str())
                .inspect_err(|_e| {
                    warn!(
                        "Invalid configuration for log level {}. Fallback to default INFO",
                        config.log_level.as_str()
                    )
                })
                .unwrap_or(LevelFilter::Info),
        )
        .init();

    info!("Loaded config {:?}", config);
    let http_bind = format!("{}:{}", config.api.host, config.api.port);

    let service: Box<dyn ContainerExecutor + 'static + Sync + Send> = match &config.orchestrator {
        Orchestrator::Docker(ref docker_config) => Box::new(DockerContainerExecutor {
            docker_config: docker_config.clone(),
            routing_config: config.routing,
            docker: Docker::connect_with_socket(&docker_config.socket, 120, API_DEFAULT_VERSION)
                .context("Can't connect to docker socket")?,
        }),
        Orchestrator::Kubernetes(ref kube_config) => Box::new(KubernetesContainerExecutor {
            kube_config: kube_config.clone(),
            routing_config: config.routing,
            client: Client::try_default().await?,
        }),
    };
    let service = domain::ReconciliationService {
        container_executor: service,
    };

    service.container_executor.ensure_routing().await?;
    // Possible feature: gracefully stop routing on shutdown hook with config

    info!("Start cleverclown http server on {}", http_bind);
    let listener = TcpListener::bind(http_bind).await.unwrap();
    axum::serve(listener, router(service)).await?;
    Ok(())
}

use std::{
    collections::HashMap,
    fs::remove_dir_all,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Error};
use async_trait::async_trait;
use bollard::{
    container::{
        Config, CreateContainerOptions, ListContainersOptions, NetworkingConfig,
        RemoveContainerOptions, StartContainerOptions,
    },
    image::{BuildImageOptions, CreateImageOptions},
    secret::{
        BuildInfoAux, CreateImageInfo, EndpointSettings, HostConfig, RestartPolicy,
        RestartPolicyNameEnum,
    },
    Docker,
};
use bytes::{BufMut, BytesMut};
use flate2::{write::GzEncoder, Compression};
use futures::{StreamExt, TryStreamExt};
use git2::Repository;
use itertools::Itertools;
use map_macro::hash_map;
use rand::{distributions::Alphanumeric, Rng};

use crate::{config::AppConfig, domain::{
    model::{Application, ApplicationSource, Container},
    port::ContainerExecutor,
}};

pub struct DockerContainerExecutor {
    pub config: AppConfig,
    pub docker: Docker,
}

#[async_trait]
impl ContainerExecutor for DockerContainerExecutor {
    async fn running(
        &self,
        application_name: String,
    ) -> Result<Vec<crate::domain::model::Container>, anyhow::Error> {
        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                filters: hash_map! {
                    "label" => vec![format!("rokku.application.name={}", application_name).as_str()]
                },
                ..Default::default()
            }))
            .await?;
        Ok(containers
            .into_iter()
            .map(|docker_container| Container {
                id: docker_container
                    .id
                    .or(docker_container
                        .names
                        .and_then(|names| names.first().cloned()))
                    .unwrap_or(application_name.clone()),
                image_id: docker_container.image.unwrap(),
                started_at: u64::try_from(docker_container.created.unwrap()).unwrap(), // TODO ???
            })
            .collect())
        // TODO should filter if container is out of date with configuration
    }

    async fn register_image(&self, application: &Application) -> Result<String, Error> {
        match application.source {
            ApplicationSource::DockerImage { ref image, pull } => {
                if pull {
                    println!("Pull image {}", image.as_str());
                    self.docker
                        .create_image(
                            Some(CreateImageOptions {
                                from_image: image.as_str(),
                                ..Default::default()
                            }),
                            None,
                            None,
                        )
                        .try_collect::<Vec<CreateImageInfo>>()
                        .await
                        .context("Error while pulling image")?;
                }
                self.docker
                    .inspect_image(image)
                    .await
                    .context("Can't detect image on docker daemon")
                    .and_then(|docker_image| {
                        docker_image
                            .id
                            .ok_or(anyhow!("Can't detect id of provided image"))
                    })
            }
            // ApplicationSource::DockerImage { ref image } => self.docker.create_image(Some(CreateImageOptions{
            //     from_image: image.as_str(),
            //     ..Default::default()
            //   }), None, None).fuse()
            //   .filter_map(|build_status|
            //     match build_status.map(|x| x.) {
            //         Ok(Some(BuildInfoAux::Default(image_id))) => {
            //             std::future::ready(Some(image_id.id))
            //         }
            //         _ => std::future::ready(None),
            //     }).select_next_some()
            //     .await
            //     .ok_or(anyhow!("Error pulling image")),
            ApplicationSource::Git { ref remote } => {
                let local_dir = format!("/tmp/{}", application.name);
                if Path::new(local_dir.as_str()).exists() {
                    remove_dir_all(Path::new(local_dir.as_str()))?;
                }
                println!("Clone git repository {}", remote);
                let _repository = Repository::clone(remote.as_str(), local_dir.as_str())?;
                self.build_image(local_dir, application.name.clone()).await
            }
            ApplicationSource::LocalRepo { ref path } => {
                self.build_image(path.clone(), application.name.clone())
                    .await
            }
        }
    }

    async fn start(&self, application: &Application, image_id: String) -> Result<Container, Error> {
        let exposed_port = match application.exposed_port {
            Some(ref port) => port.clone(),
            None => self.extract_min_exposed_port(image_id.as_str()).await?,
        };

        let config = Config {
            image: Some(image_id.clone()),
            exposed_ports: Some(hash_map! {
                format!("{}/tcp", exposed_port) => HashMap::new()
            }),
            host_config: Some(HostConfig {
                // port_bindings: Some(port_binding),
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::ON_FAILURE),
                    maximum_retry_count: Some(3),
                }),
                ..Default::default()
            }),
            labels: Some(hash_map! {
                String::from("traefik.enable") => String::from("true"),
                String::from("traefik.http.services.rokku.loadbalancer.server.port") => format!("{}", exposed_port),
                String::from("rokku.domain") => application.domain.clone().unwrap_or(application.name.clone()),
                String::from("rokku.application.name") => application.name.clone()
            }),
            networking_config: Some(NetworkingConfig {
                endpoints_config: hash_map! {
                    self.config.docker_network.clone() => EndpointSettings {
                        ..Default::default()
                    }
                },
            }),
            ..Default::default()
        };
        let container = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: format!(
                        "{}.{}",
                        application.name,
                        rand::thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(7)
                            .map(char::from)
                            .collect::<String>()
                    ),
                    ..Default::default()
                }),
                config,
            )
            .await?;
        self.docker
            .start_container(container.id.as_str(), None::<StartContainerOptions<String>>)
            .await?;

        Ok(Container {
            id: container.id,
            image_id: image_id,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backward")
                .as_secs(),
        })
    }

    async fn stop(&self, container: &Container) -> Result<(), Error> {
        self.docker
            .remove_container(
                container.id.as_str(),
                Some(RemoveContainerOptions {
                    force: true,
                    v: true,
                    ..Default::default()
                }),
            )
            .await
            .context(format!("Error while removing container {}", container.id))
    }

    async fn list_applications(&self) -> Result<Vec<String>, Error> {
        let containers = self.docker.list_containers::<String>(None).await?;

        Ok(containers
            .into_iter()
            .filter_map(|docker_container| {
                docker_container
                    .labels
                    .and_then(|labels| labels.get("rokku.application.name").cloned())
            })
            .unique()
            .collect())
    }
}

impl DockerContainerExecutor {
    async fn extract_min_exposed_port(&self, image_id: &str) -> Result<u16, Error> {
        self.docker
            .inspect_image(image_id)
            .await?
            .config
            .and_then(|c| c.exposed_ports)
            .and_then(|exposed_ports| exposed_ports.into_iter().map(|(port, _)| port).min())
            .and_then(|port| port.split("/").next().map(|str| String::from(str)))
            .ok_or(anyhow!("Can't detect exposed port for {} image. Please define it in the application configuration or add EXPOSE to image", image_id))
            .and_then(|port_as_string| port_as_string.parse::<u16>().context("Exposed port can't be parsed"))
    }

    async fn build_image(
        &self,
        local_dir: String,
        application_name: String,
    ) -> Result<String, Error> {
        let tar_gz = BytesMut::new().writer();
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all(".", local_dir.as_str())?;

        let tar_gz = tar.into_inner()?.finish()?;

        println!("Build image {}", application_name.as_str());
        self.docker
            .build_image(
                BuildImageOptions {
                    dockerfile: "Dockerfile",
                    t: application_name.as_str(),
                    version: bollard::image::BuilderVersion::BuilderBuildKit,
                    pull: true,
                    session: Some("buildx-session".into()),
                    ..Default::default()
                },
                None,
                Some(tar_gz.into_inner().freeze()),
            )
            .fuse()
            .filter_map(|info| {
                match info.map(|x| x.aux) {
                    Ok(Some(BuildInfoAux::BuildKit(response))) => {
                        for vertex in response.vertexes {
                            if vertex.completed.is_some() {
                                println!("Buildx => [Vertex] {}", vertex.name)
                            }
                        }
                        for status in response.statuses {
                            if status.completed.is_some() {
                                println!("Buildx => [Status] {}", status.id)
                            }
                        }
                        // println!("Buildx => {:?}", response.vertexes.iter());
                        std::future::ready(None)
                    }
                    Ok(Some(BuildInfoAux::Default(image_id))) => {
                        std::future::ready(Some(image_id.id))
                    }
                    _ => std::future::ready(None),
                }
            })
            .select_next_some()
            .await
            .ok_or(anyhow!("Image built but cannot detect image id"))
    }
}

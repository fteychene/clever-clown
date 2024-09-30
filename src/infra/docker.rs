use std::{
    collections::HashMap, fs::remove_dir_all, path::Path, time::{SystemTime, UNIX_EPOCH}
};

use anyhow::{anyhow, Context, Error};
use async_trait::async_trait;
use bollard::{
    container::{
        AttachContainerOptions, AttachContainerResults, Config, CreateContainerOptions,
        ListContainersOptions, LogOutput, NetworkingConfig, RemoveContainerOptions,
        StartContainerOptions, UploadToContainerOptions,
    }, image::{BuildImageOptions, CreateImageOptions}, network::{CreateNetworkOptions, ListNetworksOptions}, secret::{
        BuildInfoAux, CreateImageInfo, EndpointSettings, HostConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum
    }, Docker
};
use bytes::{BufMut, BytesMut};
use flate2::{write::GzEncoder, Compression};
use futures::{StreamExt, TryStreamExt};
use git2::Repository;
use itertools::Itertools;
use log::{info, warn};
use map_macro::hash_map;
use rand::{distributions::Alphanumeric, Rng};

use crate::{
    config::{DockerConfig, RoutingConfig},
    domain::{
        model::{Application, ApplicationSource, Container},
        port::ContainerExecutor,
    },
};

pub struct DockerContainerExecutor {
    pub docker_config: DockerConfig,
    pub routing_config: RoutingConfig,
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
                    "label" => vec![format!("cleverclown.application.name={}", application_name).as_str()]
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
    }

    async fn register_image(&self, application: &Application) -> Result<String, Error> {
        match application.source {
            ApplicationSource::DockerImage { ref image, pull } => {
                if pull {
                    info!("Pull image {}", image.as_str());
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
            ApplicationSource::Git {
                ref remote,
                ref dockerfile,
            } => {
                let local_dir = format!("{}/{}", self.docker_config.source_directory, application.name);
                if Path::new(local_dir.as_str()).exists() {
                    remove_dir_all(Path::new(local_dir.as_str()))?;
                }
                info!("Clone git repository {}", remote);
                let _repository = Repository::clone(remote.as_str(), local_dir.as_str())?;
                match dockerfile {
                    Some(ref dockerfile) => {
                        self.build_docker_image(
                            local_dir,
                            application.name.clone(),
                            dockerfile.clone(),
                        )
                        .await
                    }
                    None => {
                        self.build_image_buildpack(local_dir, application.name.clone())
                            .await
                    }
                }
            }
            ApplicationSource::LocalRepo {
                ref path,
                ref dockerfile,
            } => match dockerfile {
                Some(ref dockerfile) => {
                    self.build_docker_image(
                        path.clone(),
                        application.name.clone(),
                        dockerfile.clone(),
                    )
                    .await
                }
                None => {
                    self.build_image_buildpack(path.clone(), application.name.clone())
                        .await
                }
            },
        }
    }

    async fn register_application(&self, application: &Application, _image_id: String) -> Result<Vec<Container>, Error> {
        // Docker runtime doesn't support application definition
        self.running(application.name.clone()).await
    }

    async fn delete_application(&self, _application: String) -> Result<(), Error> {
        // Docker runtime doesn't support application definition
        Ok(())
    }

    async fn start_instance(&self, application: &Application, image_id: String) -> Result<Container, Error> {
        let exposed_port = match application
            .configuration
            .as_ref()
            .and_then(|configuration| configuration.exposed_port)
        {
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
                format!("traefik.http.routers.{}.rule", application.name) => format!("Host(`{}.{}`)",  application.configuration.as_ref().and_then(|configuration| configuration.domain.clone()).unwrap_or(application.name.clone()), self.routing_config.domain),
                String::from("traefik.http.services.cleverclown.loadbalancer.server.port") => format!("{}", exposed_port),
                String::from("cleverclown.domain") => application.configuration.as_ref().and_then(|configuration| configuration.domain.clone()).unwrap_or(application.name.clone()),
                String::from("cleverclown.application.name") => application.name.clone()
            }),
            networking_config: Some(NetworkingConfig {
                endpoints_config: hash_map! {
                    self.docker_config.network.clone() => EndpointSettings {
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

    async fn stop_instance(&self, _application: String, container: &Container) -> Result<(), Error> {
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
                    .and_then(|labels| labels.get("cleverclown.application.name").cloned())
            })
            .unique()
            .collect())
    }

    async fn ensure_routing(&self) -> Result<(), Error> {
        let network = self.docker.list_networks(Some(ListNetworksOptions{
            filters: hash_map! { "name" => vec![self.docker_config.network.as_str()]}
        })).await?;
        
        if network.is_empty() {
            info!("Configured network {} is missing. Create network", self.docker_config.network);
            self.docker.create_network(CreateNetworkOptions {
                name: self.docker_config.network.as_str(),
                driver: "bridge",
                ..Default::default()
            }).await?;
        }
        
        let traefik_container_name = "cleverclown_traefik";
        let container = match self.docker.inspect_container(traefik_container_name, None).await { //TODO should unwrap_or but future on op
            Ok(traefik_container) => {
                info!("Traefik http routing continer detected {}", traefik_container.id.clone().unwrap());
                traefik_container
            }, 
            Err(_) => { // TODO should check if error is just not existing
                info!("No routing traefik container detected, starting it");
                let mut exposed_ports = hash_map! {
                    "80/tcp".to_string() => HashMap::new(),
                };
                let mut port_binding = hash_map! {
                    "80/tcp".to_string() => Some(vec![PortBinding { host_port: Some("80".to_string()), host_ip: None }])
                };
                let mut environment = vec![
                    format!("TRAEFIK_PROVIDERS_DOCKER_NETWORK={}", self.docker_config.network),
                    format!("TRAEFIK_PROVIDERS_DOCKER_EXPOSEDBYDEFAULT={}", "false"),
                    format!("TRAEFIK_LOG_LEVEL={}", "info"),
                    format!("TRAEFIK_LOG_NOCOLOR={}", "true"),
                    format!("TRAEFIK_PROVIDERS_DOCKER_ENDPOINT=unix://{}", self.docker_config.socket)
                ];
                if self.routing_config.dashboard {
                    exposed_ports.insert("8080/tcp".to_string(), HashMap::new());
                    port_binding.insert("8080/tcp".to_string(), Some(vec![PortBinding { host_port: Some("8080".to_string()), host_ip: None }]));
                    environment.push("TRAEFIK_API_INSECURE=true".to_string());
                }
                let traefik_config = Config {
                    image: Some("traefik:v3.1".to_string()),
                    env: Some(environment),
                    exposed_ports: Some(exposed_ports),
                    host_config: Some(HostConfig {
                        port_bindings: Some(port_binding),
                        binds: Some(vec![format!("{}:{}", self.docker_config.socket, self.docker_config.socket)]),
                        restart_policy: Some(RestartPolicy {
                            name: Some(RestartPolicyNameEnum::ON_FAILURE),
                            maximum_retry_count: Some(3),
                        }),
                        ..Default::default()
                    }),
                    networking_config: Some(NetworkingConfig { endpoints_config: hash_map! { 
                        self.docker_config.network.clone() => EndpointSettings { ..Default::default() } 
                    }}), 
                    ..Default::default()
                };
                let container_name = self.docker.create_container(Some(CreateContainerOptions{
                    name: traefik_container_name,
                    platform: None,
                }), traefik_config).await?;
                info!("Created container {}", container_name.id);
                self.docker.inspect_container(&container_name.id.as_str(), None).await.context("Error while inspecting newly created traefik container")?
            }
        };
        // TODO should check config is up to date
        if !container.state.and_then(|state| state.running).unwrap_or(false) {
            info!("Starting traefik container");
            self.docker.start_container::<String>(container.id.unwrap().as_str(), None).await.context("Error starting traefik container for routing")
        } else {
            Ok(())
        }
        

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

    async fn build_docker_image(
        &self,
        local_dir: String,
        application_name: String,
        dockerfile: String,
    ) -> Result<String, Error> {
        let tar_gz = BytesMut::new().writer();
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all(".", local_dir.as_str())?;

        let tar_gz = tar.into_inner()?.finish()?;

        info!("Build image {}", application_name.as_str());
        self.docker
            .build_image(
                BuildImageOptions {
                    dockerfile: dockerfile.as_str(),
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
            .filter_map(|info| match info.map(|x| x.aux) {
                Ok(Some(BuildInfoAux::BuildKit(response))) => {
                    for vertex in response.vertexes {
                        if vertex.completed.is_some() {
                            info!("Buildx => [Vertex] {}", vertex.name)
                        }
                    }
                    for status in response.statuses {
                        if status.completed.is_some() {
                            info!("Buildx => [Status] {}", status.id)
                        }
                    }
                    std::future::ready(None)
                }
                Ok(Some(BuildInfoAux::Default(image_id))) => std::future::ready(Some(image_id.id)),
                _ => std::future::ready(None),
            })
            .select_next_some()
            .await
            .ok_or(anyhow!("Image built but cannot detect image id"))
    }

    async fn build_image_buildpack(
        &self,
        local_dir: String,
        application_name: String,
    ) -> Result<String, Error> {
        let buildpack_config = Config {
            image: Some("buildpacksio/pack"),
            cmd: Some(vec![
                "build",
                application_name.as_str(),
                "--builder",
                "heroku/builder:24",
            ]),
            working_dir: Some("/workspace"),
            host_config: Some(HostConfig {
                binds: Some(vec![
                    format!("{}:/var/run/docker.sock", self.docker_config.socket),
                    // format!("{}:/workspace", buildpack_volume), // TODO rework this linking in docker mode
                ]),
                ..Default::default()
            }),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let buildpack_container_id = self
            .docker
            .create_container::<&str, &str>(None, buildpack_config)
            .await?
            .id;

        let tar_gz = BytesMut::new().writer();
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all(".", local_dir.as_str())?;
        let tar_gz = tar.into_inner()?.finish()?;

        self.docker.upload_to_container(buildpack_container_id.as_str(), Some(UploadToContainerOptions {
            path: "/workspace",
            ..Default::default()
        }), tar_gz.into_inner().freeze()).await?;

        self.docker
            .start_container::<String>(&buildpack_container_id, None)
            .await?;

        let AttachContainerResults { mut output, .. } = self
            .docker
            .attach_container(
                &buildpack_container_id,
                Some(AttachContainerOptions::<String> {
                    stdout: Some(true),
                    stderr: Some(true),
                    stream: Some(true),
                    ..Default::default()
                }),
            )
            .await?;
        while let Some(Ok(output)) = output.next().await {
            match output {
                LogOutput::StdOut { message } => info!("Buildpack => {:?}", message),
                LogOutput::StdErr { message } => warn!("Buildpack => {:?}", message),
                _ => {}
            }
        }
        self.docker.remove_container(buildpack_container_id.as_str(), None).await?;
        Ok(application_name)
    }
}

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Error};
use axum::async_trait;
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{Pod, Service},
    networking::v1::Ingress,
};
use kube::{
    api::{DeleteParams, ListParams, PostParams},
    Api, Client,
};
use log::info;
use serde_json::json;

use crate::{
    config::{KubernetesConfig, RoutingConfig},
    domain::{
        model::{Application, Container},
        port::ContainerExecutor,
    },
};

pub struct KubernetesContainerExecutor {
    pub kube_config: KubernetesConfig,
    pub routing_config: RoutingConfig,
    pub client: Client,
}

#[async_trait]
impl ContainerExecutor for KubernetesContainerExecutor {
    async fn register_image(&self, application: &Application) -> Result<String, Error> {
        match application.source {
            crate::domain::model::ApplicationSource::DockerImage { ref image, pull: _ } => {
                Ok(image.clone())
            }
            _ => Err(anyhow!("Kubernetes runtime only support DockerImage application source")),
        }
    }

    async fn running(&self, application: String) -> Result<Vec<Container>, Error> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);
        let deployments: Api<Deployment> =
            Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);

        let app_deployment = deployments
            .list(&ListParams {
                label_selector: Some(format!("cleverclown.app={}", application)),
                ..Default::default()
            })
            .await?;
        if app_deployment.items.len() == 0 {
            return Ok(vec![]);
        }
        let pods = pods
            .list(&ListParams {
                label_selector: Some(format!("cleverclown.app={}", application)),
                ..Default::default()
            })
            .await?;
        Ok(pods
            .into_iter() // TODO manage unwraps
            .map(|pod| Container {
                id: pod.metadata.name.unwrap(),
                started_at: pod
                    .metadata
                    .creation_timestamp
                    .map(|x| wrap_to_u64(x.0.timestamp_millis()))
                    .unwrap(),
                image_id: pod
                    .spec
                    .and_then(|spec| spec.containers.get(0).cloned())
                    .and_then(|container| container.image)
                    .unwrap(),
            })
            .collect())
    }

    async fn register_application(
        &self,
        application: &Application,
        image_id: String,
    ) -> Result<Vec<Container>, Error> {
        let deployments: Api<Deployment> =
            Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);

        let app_deployment: Deployment = serde_json::from_value(json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": application.name.clone(),
                "labels": {
                    "cleverclown.app": application.name.clone(),
                },
            },
            "spec": {
                "replicas": application.configuration.as_ref().and_then(|cfg| cfg.replicas).clone(),
                "selector": {
                    "matchLabels": {
                        "cleverclown.app": application.name.clone(),
                    },
                },
                "template": {
                    "metadata": {
                        "labels": {
                            "cleverclown.app": application.name.clone(),
                        },
                    },
                    "spec": {
                        "containers": [
                            {
                            "name": "application",
                            "image": image_id,
                            "ports": [
                                {
                                    "containerPort" : application.configuration.as_ref().and_then(|cfg| cfg.exposed_port).clone()
                                }
                            ]
                            }
                        ]
                    }
                },
            }
        }))?;
        if deployments
            .list(&ListParams {
                label_selector: Some(format!("cleverclown.app={}", application.name.as_str())),
                ..Default::default()
            })
            .await?
            .items
            .len()
            > 0
        {
            deployments
                .replace(
                    application.name.as_str(),
                    &PostParams {
                        ..Default::default()
                    },
                    &app_deployment,
                )
                .await?;
        } else {
            deployments
                .create(
                    &PostParams {
                        ..Default::default()
                    },
                    &app_deployment,
                )
                .await?;
        }

        let services: Api<Service> =
            Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);
        let service: Service = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Service",
            "metadata": {
                "name": application.name.clone(),
                "labels": {
                    "cleverclown.app": application.name.clone(),
                },
            },
            "spec": {
                "ports": [
                    {
                        "name": "app",
                     "port": application.configuration.as_ref().and_then(|cfg| cfg.exposed_port).clone(),
                     "targetPort": application.configuration.as_ref().and_then(|cfg| cfg.exposed_port).clone()
                    }
                ],
            "selector": {
                "cleverclown.app": application.name.clone()
            }
        }
        }))?;
        if services
            .list(&ListParams {
                label_selector: Some(format!("cleverclown.app={}", application.name.as_str())),
                ..Default::default()
            })
            .await?
            .items
            .len()
            > 0
        {
            services
                .replace(
                    application.name.as_str(),
                    &PostParams {
                        ..Default::default()
                    },
                    &service,
                )
                .await?;
        } else {
            services
                .create(
                    &PostParams {
                        ..Default::default()
                    },
                    &service,
                )
                .await?;
        }

        let ingresses: Api<Ingress> =
            Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);
        let ingress: Ingress = serde_json::from_value(json!({
                "apiVersion": "networking.k8s.io/v1",
                "kind": "Ingress",
                "metadata": {
                    "name": application.name.clone(),
                    "labels": {
                        "cleverclown.app": application.name.clone(),

                    },
                },
                "spec": {
                    "rules":[
                        {
                            "host": format!("{}.{}", application.configuration.as_ref().and_then(|cfg| cfg.domain.clone()).unwrap_or(application.name.clone()),
                                self.routing_config.domain),
                            "http": {
                                "paths": [
                                {"path": "/",
                                "pathType": "Prefix",
                                "backend": {
                                    "service": {
                                        "name": application.name.clone(),
                                        "port": {
                                            "name": "app"
                                        }
                                    }
                                }}
                                ]
                            }
                        }
                    ]
                }
        }))?;
        if ingresses
            .list(&ListParams {
                label_selector: Some(format!("cleverclown.app={}", application.name.as_str())),
                ..Default::default()
            })
            .await?
            .items
            .len()
            > 0
        {
            ingresses
                .replace(
                    application.name.as_str(),
                    &PostParams {
                        ..Default::default()
                    },
                    &ingress,
                )
                .await?;
        } else {
            ingresses
                .create(
                    &PostParams {
                        ..Default::default()
                    },
                    &ingress,
                )
                .await?;
        }

        let mut instances = self.running(application.name.clone()).await?;
        let started = Instant::now();
        while instances.len()
            < application
                .configuration
                .as_ref()
                .and_then(|config| config.replicas)
                .unwrap_or(1)
                .into()
            && started.elapsed().as_millis() < 5000
        {
            instances = self.running(application.name.clone()).await?;
        }
        if started.elapsed().as_millis() >= 5000 {
            Err(anyhow!("Deployment registered in Kubernetes but pods aren't detected after 5s"))
        } else {
            Ok(instances)
        }
    }

    async fn delete_application(&self, application: String) -> Result<(), Error> {
        let deployments: Api<Deployment> =
            Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);
        let services: Api<Service> =
            Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);
        let ingresses: Api<Ingress> =
            Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);

        let _ = ingresses
            .delete(
                application.as_str(),
                &DeleteParams {
                    ..Default::default()
                },
            )
            .await;

        let _ = services
            .delete(
                application.as_str(),
                &DeleteParams {
                    ..Default::default()
                },
            )
            .await;
        let _ = deployments
            .delete(
                application.as_str(),
                &DeleteParams {
                    ..Default::default()
                },
            )
            .await;

        Ok(())
    }

    async fn start_instance(
        &self,
        application: &Application,
        image_id: String,
    ) -> Result<Container, Error> {
        Ok(Container {
            id: application.name.clone(),
            image_id: image_id,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backward")
                .as_secs(),
        })
    }

    async fn stop_instance(
        &self,
        _application: String,
        _container: &Container,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn list_applications(&self) -> Result<Vec<String>, Error> {
        let deployments: Api<Deployment> =
            Api::namespaced(self.client.clone(), &self.kube_config.app_namespace);
        let applications = deployments
            .list(&ListParams {
                ..Default::default()
            })
            .await?;
        Ok(applications
            .into_iter()
            .filter(|deployment| {
                deployment
                    .metadata
                    .labels
                    .as_ref()
                    .filter(|labels| labels.contains_key("cleverclown.app"))
                    .is_some()
            })
            .map(|deployment| deployment.metadata.name.unwrap())
            .collect())
    }

    async fn ensure_routing(&self) -> Result<(), Error> {
        info!("TODO - Check traefik is installed");
        Ok(())
    }
}

pub fn wrap_to_u64(x: i64) -> u64 {
    (x as u64).wrapping_add(u64::MAX / 2 + 1)
}

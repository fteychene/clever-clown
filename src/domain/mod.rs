use anyhow::{anyhow, Error};
use log::info;
use model::{Application, Container};
use port::ContainerExecutor;
use split_iter::Splittable;

pub mod model;
pub mod port;

pub struct ReconciliationService {
    // pub application_repository: Box<dyn ApplicationRepository>,
    pub container_executor: Box<dyn ContainerExecutor + 'static + Sync + Send>,
}

pub enum Event {
    Deploy(Application),
    Destroy(String),
}

pub async fn reconcile(event: Event, service: &ReconciliationService) -> Result<(), Error> {
    match event {
        Event::Deploy(application) => {
            let image_id = service
                .container_executor
                .register_image(&application)
                .await?;
            info!("Application image detected : {}", image_id);
            let app_containers = service
                .container_executor
                .register_application(&application, image_id.clone())
                .await?;
            let (outdated_containers, valid_containers) = app_containers
                .into_iter()
                .split(|container| container.image_id.eq(&image_id));
            let outdated_containers: Vec<Container> = outdated_containers.collect();
            if !outdated_containers.is_empty() {
                info!(
                    "Detected {} outdated container runnning. Will be stopped as rolling update",
                    outdated_containers.len()
                );
            }
            // Could be reintroduced for a down then start rolling strategy
            // for outdated in outdated_containers {
            //     info!("Detected outdated container running {}. Stopping container...", outdated.id);
            //     service.container_executor.stop(&outdated).await?;
            // }
            let mut app_containers: Vec<Container> = valid_containers.collect();
            let target_replicas = usize::from(
                application
                    .configuration
                    .as_ref()
                    .and_then(|configuration| configuration.replicas)
                    .unwrap_or(1),
            );
            if target_replicas > app_containers.len() {
                info!(
                    "{} running instances. Starting {} instances",
                    app_containers.len(),
                    target_replicas - app_containers.len()
                );
                let mut outdated_containers = outdated_containers.into_iter();
                for _ in app_containers.len()..target_replicas {
                    let container = service
                        .container_executor
                        .start_instance(&application, image_id.clone())
                        .await?;
                    info!("Instance {} started", container.id);
                    if let Some(outdated) = outdated_containers.next() {
                        service
                            .container_executor
                            .stop_instance(application.name.clone(), &outdated)
                            .await?;
                        info!("Outdated instance {} stopped", outdated.id);
                    }
                }
                for outdated in outdated_containers {
                    service
                        .container_executor
                        .stop_instance(application.name.clone(), &outdated)
                        .await?;
                    info!("Outdated instance {} stopped", outdated.id);
                }
            } else if target_replicas == app_containers.len() {
                info!("Application is up-to-date")
            } else {
                info!(
                    "{} running instances. Downscaling to {} instances",
                    app_containers.len(),
                    target_replicas
                );
                app_containers.sort_by(|a: &model::Container, b| a.started_at.cmp(&b.started_at));
                for container in app_containers
                    .iter()
                    .take(app_containers.len() - target_replicas)
                {
                    service
                        .container_executor
                        .stop_instance(application.name.clone(), container)
                        .await?;
                    info!("Instance {} deleted", container.id);
                }
            }
            Ok(())
        }
        Event::Destroy(application_name) => {
            let containers = service
                .container_executor
                .running(application_name.clone())
                .await?;
            if containers.len() < 1 {
                return Err(anyhow!("Application {} is not running", application_name));
            }
            futures::future::join_all(containers.iter().map(|container| {
                service
                    .container_executor
                    .stop_instance(application_name.clone(), container)
            }))
            .await
            .into_iter()
            .collect::<Result<(), Error>>()?;
            service
                .container_executor
                .delete_application(application_name.clone())
                .await
        }
    }
}

pub async fn list_applications(
    reconciliation_service: &ReconciliationService,
) -> Result<Vec<String>, Error> {
    reconciliation_service
        .container_executor
        .list_applications()
        .await
}

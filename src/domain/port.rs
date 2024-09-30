use super::model::{Application, Container};
use anyhow::Error;
use async_trait::async_trait;

#[async_trait]
pub trait ContainerExecutor {
    async fn register_image(&self, application: &Application) -> Result<String, Error>;

    async fn register_application(&self, application: &Application, image_id: String) -> Result<Vec<Container>, Error>;

    async fn delete_application(&self, application: String) -> Result<(), Error>;

    async fn running(&self, application: String) -> Result<Vec<Container>, Error>;

    async fn start_instance(&self, application: &Application, image_id: String) -> Result<Container, Error>;

    async fn stop_instance(&self, application_name: String, container: &Container) -> Result<(), Error>;

    async fn list_applications(&self) -> Result<Vec<String>, Error>;

    async fn ensure_routing(&self) -> Result<(), Error>;
}

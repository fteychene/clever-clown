use anyhow::Error;
use async_trait::async_trait;
use super::model::{Application, Container};

#[async_trait]
pub trait ContainerExecutor {

    async fn register_image(&self, application: &Application) -> Result<String, Error>;

    async fn running(&self, application: String) -> Result<Vec<Container>, Error>;

    async fn start(&self, application: &Application, image_id: String) -> Result<Container, Error>;

    async fn stop(&self, container: &Container) -> Result<(), Error>;

    async fn list_applications(&self) -> Result<Vec<String>, Error>;
}

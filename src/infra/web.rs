use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use log::error;

use crate::domain::{model::Application, reconcile, Event, ReconciliationService};

pub fn router(reconciliation: ReconciliationService) -> Router {
    Router::new()
        .route("/", get(list_applications))
        .route("/", post(deploy_application))
        .route("/:app_name", delete(destroy_application))
        .with_state(Arc::new(reconciliation))
}

async fn list_applications(State(service): State<Arc<ReconciliationService>>) -> impl IntoResponse {
    crate::domain::list_applications(&service)
        .await
        .map(|applications| Json(applications))
        .map_err(|e| {
            error!("Error during list_application {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {e}"),
            )
        })
}

async fn deploy_application(
    State(service): State<Arc<ReconciliationService>>,
    Json(payload): Json<Application>,
) -> impl IntoResponse {
    reconcile(Event::Deploy(payload), service.as_ref())
        .await
        .map(|_| (StatusCode::OK, "Application deployed"))
        .map_err(|e| {
            error!("Error during deploy_application {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {e}"),
            )
        })
}

async fn destroy_application(
    State(service): State<Arc<ReconciliationService>>,
    Path(app_name): Path<String>,
) -> impl IntoResponse {
    reconcile(Event::Destroy(app_name), service.as_ref())
        .await
        .map(|_| (StatusCode::OK, "Application destoyed"))
        .map_err(|e| {
            error!("Error during destroy_application {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {e}"),
            )
        })
}

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

pub mod app_routes;
pub mod job_routes;

#[derive(Debug, Serialize)]
struct ApiResponse<T>
where
    T: Serialize,
{
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        Self { data: Some(data), error: None }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self { data: None, error: Some(message.into()) }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let status = if self.error.is_some() { StatusCode::INTERNAL_SERVER_ERROR } else { StatusCode::OK };

        let json = Json(self);

        (status, json).into_response()
    }
}

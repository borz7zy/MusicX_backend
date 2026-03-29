use axum::{Extension, Json, response::IntoResponse};
use crate::config::oauth::Claims;
use crate::error::ApiError;
use crate::models::user::{CreateUserRequest, CreateUserResponse};

pub async fn create_user(
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.name.is_empty() {
        return Err(ApiError::InvalidInput("Name is required".into()));
    }

    let username = claims.preferred_username.unwrap_or(claims.sub);

    Ok(Json(CreateUserResponse {
        message: format!("User {} created by {}", payload.name, username),
    }))
}

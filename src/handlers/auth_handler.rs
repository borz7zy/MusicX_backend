use axum::{Json, extract::State, response::IntoResponse, http::StatusCode};
use crate::error::ApiError;
use crate::models::auth_model::{RegisterRequest, RegisterResponse, TokenRequest, TokenResponse, RefreshTokenRequest};
use crate::state::AppState;

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if payload.username.is_empty() {
        return Err(ApiError::InvalidInput("Username is required".into()));
    }
    if payload.email.is_empty() || !payload.email.contains('@') {
        return Err(ApiError::InvalidInput("Valid email is required".into()));
    }
    if payload.password.len() < 8 {
        return Err(ApiError::InvalidInput(
            "Password must be at least 8 characters".into(),
        ));
    }

    state
        .keycloak
        .register_user(&payload.username, &payload.email, &payload.password)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            message: "User registered successfully".into(),
            username: payload.username,
        }),
    ))
}

pub async fn token(
    State(state): State<AppState>,
    Json(payload): Json<TokenRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(ApiError::InvalidInput(
            "Username and password are required".into(),
        ));
    }

    let token_response = state
        .keycloak
        .get_token(&payload.username, &payload.password)
        .await?;

    Ok(Json(token_response))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    if payload.refresh_token.is_empty() {
        return Err(ApiError::InvalidInput("refresh_token is required".into()));
    }

    let token_response = state
        .keycloak
        .refresh_token(&payload.refresh_token)
        .await?;

    Ok(Json(token_response))
}

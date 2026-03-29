use axum::{Router, extract::DefaultBodyLimit, middleware, routing::{delete, get, patch, post}};
use std::sync::Arc;
use crate::{handlers::{audio, auth_handler, health, users}, services::storage};
use crate::config::{minio::init_minio, db::init_db, oauth::OAuthConfig, keycloak::KeycloakClient};
use crate::middleware::auth::require_auth;
use crate::state::AppState;

pub async fn create_app() -> Router {
    let state = AppState {
        minio_client: init_minio(),
        db: init_db().await,
        oauth: Arc::new(OAuthConfig::init().await),
        keycloak: Arc::new(KeycloakClient::init()),
    };
    
    storage::ensure_bucket(&state.minio_client).await;
    
    let public = Router::new()
        .route("/auth/register", post(auth_handler::register))
        .route("/auth/token", post(auth_handler::token))
        .route("/auth/refresh", post(auth_handler::refresh))
        .route("/audio/search", get(audio::search));

    let protected =Router::new()
        .route("/users", post(users::create_user))
        .route("/audio", post(audio::upload))
        .route("/audio", get(audio::my_list))
        .route("/audio/{id}/stream", get(audio::stream_url))
        .route("/audio/{id}/privacy", patch(audio::update_privacy))
        .route("/audio/{id}/collect", post(audio::add_to_collection))
        .route("/audio/{id}/collect", delete(audio::remove_from_collection))
        .route("/audio/{id}", delete(audio::delete))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/ping.txt", get(health::health_check))
        .merge(public)
        .merge(protected)
        .layer(DefaultBodyLimit::max(1024*1024*1024))
        .with_state(state)
}
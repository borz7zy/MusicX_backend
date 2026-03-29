use std::sync::Arc;
use aws_sdk_s3::Client as S3Client;
use crate::config::oauth::OAuthConfig;
use crate::config::keycloak::KeycloakClient;

#[derive(Clone)]
pub struct AppState {
    pub minio_client: S3Client,
    pub db: sqlx::PgPool,
    pub oauth: Arc<OAuthConfig>,
    pub keycloak: Arc<KeycloakClient>,
}

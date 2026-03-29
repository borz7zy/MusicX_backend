use reqwest::Client as HttpClient;
use serde_json::json;
use std::env;
use crate::error::ApiError;
use crate::models::auth_model::{TokenResponse};

pub struct KeycloakClient {
    pub http: HttpClient,
    pub base_url: String,
    pub realm: String,
    pub client_id: String,
    pub client_secret: String,
    pub admin_user: String,
    pub admin_password: String,
}

impl KeycloakClient {
    pub fn init() -> Self {
        Self {
            http: HttpClient::new(),
            base_url: env::var("KEYCLOAK_URL").expect("KEYCLOAK_URL environment variable not set"),
            realm: env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM environment variable not set"),
            client_id: env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID environment variable not set"),
            client_secret: env::var("KEYCLOAK_CLIENT_SECRET").expect("KEYCLOAK_CLIENT_SECRET environment variable not set"),
            admin_user: env::var("KEYCLOAK_ADMIN").expect("KEYCLOAK_ADMIN environment variable not set"),
            admin_password: env::var("KEYCLOAK_ADMIN_PASSWORD").expect("KEYCLOAK_ADMIN_PASSWORD environment variable not set"),
        }
    }

    async fn admin_token(&self) -> Result<String, ApiError> {
        let url = format!(
            "{}/realms/master/protocol/openid-connect/token",
            self.base_url
        );

        let resp = self
            .http
            .post(&url)
            .form(&[
                ("grant_type", "password"),
                ("client_id", "admin-cli"),
                ("username", &self.admin_user),
                ("password", &self.admin_password),
            ])
            .send()
            .await
            .map_err(|_| ApiError::InternalError)?;

        if !resp.status().is_success() {
            return Err(ApiError::InternalError);
        }

        let body: serde_json::Value = resp.json().await.map_err(|_| ApiError::InternalError)?;

        body["access_token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(ApiError::InternalError)
    }

    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<(), ApiError> {
        let admin_token = self.admin_token().await?;

        let url = format!(
            "{}/admin/realms/{}/users",
            self.base_url, self.realm
        );

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&admin_token)
            .json(&json!({
                "username": username,
                "email": email,
                "enabled": true,
                "emailVerified": true,
                "credentials": [{
                    "type": "password",
                    "value": password,
                    "temporary": false
                }]
            }))
            .send()
            .await
            .map_err(|_| ApiError::InternalError)?;

        match resp.status().as_u16() {
            201 => Ok(()),
            409 => Err(ApiError::InvalidInput(
                "User with this username or email already exists".into(),
            )),
            _ => Err(ApiError::InternalError),
        }
    }

    pub async fn get_token(
        &self,
        username: &str,
        password: &str,
    ) -> Result<TokenResponse, ApiError> {
        let url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.base_url, self.realm
        );

        let resp = self
            .http
            .post(&url)
            .form(&[
                ("grant_type", "password"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("username", username),
                ("password", password),
            ])
            .send()
            .await
            .map_err(|_| ApiError::InternalError)?;

        if resp.status() == 401 {
            return Err(ApiError::InvalidInput("Invalid username or password".into()));
        }

        if !resp.status().is_success() {
            return Err(ApiError::InternalError);
        }

        resp.json::<TokenResponse>()
            .await
            .map_err(|_| ApiError::InternalError)
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, ApiError> {
        let url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.base_url, self.realm
        );

        let resp = self
            .http
            .post(&url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await
            .map_err(|_| ApiError::InternalError)?;

        if resp.status() == 400 {
            return Err(ApiError::InvalidInput("Refresh token is expired or invalid".into()));
        }

        if !resp.status().is_success() {
            return Err(ApiError::InternalError);
        }

        resp.json::<TokenResponse>()
            .await
            .map_err(|e| {eprintln!("Error parsing token response: {:?}", e); ApiError::InternalError})
    }

}

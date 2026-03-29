use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, TokenData, Validation,
};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::env;
use crate::error::ApiError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

pub struct OAuthConfig {
    pub jwks: JwkSet,
    pub issuer: String,
}

impl OAuthConfig {
    pub async fn init() -> Self {
        let keycloak_url = env::var("KEYCLOAK_URL").expect("KEYCLOAK_URL environment variable not set");
        let realm = env::var("KEYCLOAK_REALM").expect("KEYCLOAK_REALM environment variable not set");

        let issuer = format!("{}/realms/{}", keycloak_url, realm);
        let jwks_url = format!("{}/protocol/openid-connect/certs", issuer);

        let http = HttpClient::new();
        let jwks: JwkSet = http
            .get(&jwks_url)
            .send()
            .await
            .expect("Failed to fetch JWKS from Keycloak. Is Keycloak running?")
            .json()
            .await
            .expect("Failed to parse JWKS response");

        println!("OAuth2: JWKS loaded from {}", jwks_url);

        Self { jwks, issuer }
    }

    pub fn validate_token(&self, token: &str) -> Result<TokenData<Claims>, ApiError> {
        let header = decode_header(token).map_err(|_| ApiError::Unauthorized)?;

        let kid = header.kid.ok_or(ApiError::Unauthorized)?;

        let jwk = self.jwks.find(&kid).ok_or(ApiError::Unauthorized)?;

        let decoding_key = match &jwk.algorithm {
            AlgorithmParameters::RSA(rsa) => {
                DecodingKey::from_rsa_components(&rsa.n, &rsa.e)
                    .map_err(|_| ApiError::Unauthorized)?
            }
            _ => return Err(ApiError::Unauthorized),
        };

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.validate_aud = false;

        decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|_| ApiError::Unauthorized)
    }
}

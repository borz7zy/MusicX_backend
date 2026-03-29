use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub message: String,
}
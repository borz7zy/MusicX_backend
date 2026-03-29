use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::header::AUTHORIZATION,
};
use crate::error::ApiError;
use crate::state::AppState;

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;

    let token_data = state.oauth.validate_token(token)?;

    req.extensions_mut().insert(token_data.claims);

    Ok(next.run(req).await)
}

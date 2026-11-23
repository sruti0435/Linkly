use crate::{
    error::ApiError,
    models::{AuthUser, Credentials},
    state::AppState,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::error::DatabaseError;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub fn hash_password(password: &str) -> Result<String, password_hash::Error> {
    let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes())?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    PasswordHash::new(password_hash).ok().is_some_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

pub fn create_token(user_id: Uuid, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expires_at = Utc::now() + Duration::days(7);
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            exp: expires_at.timestamp() as usize,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub async fn authenticate(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "missing bearer token",
        ))?;

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ApiError::new(StatusCode::UNAUTHORIZED, "invalid or expired token"))?
    .claims;
    let id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiError::new(StatusCode::UNAUTHORIZED, "invalid token subject"))?;

    request.extensions_mut().insert(AuthUser { id });
    Ok(next.run(request).await)
}

pub fn is_duplicate_key(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(DatabaseError::code)
        .is_some_and(|code| code == "23505")
}

pub fn bad_credentials() -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, "invalid email or password")
}

pub fn internal_auth_error(error: impl std::fmt::Display) -> ApiError {
    tracing::error!(%error, "authentication operation failed");
    ApiError::internal()
}

pub fn invalid_credentials_input() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "email and password must be valid; password must be at least 8 characters",
    )
}

pub fn validate_credentials(credentials: &Credentials) -> bool {
    let email = credentials.email.trim();
    email.len() <= 254
        && email.contains('@')
        && !email.chars().any(char::is_whitespace)
        && (8..=128).contains(&credentials.password.len())
}

#[cfg(test)]
mod tests {
    use super::{hash_password, validate_credentials, verify_password};
    use crate::models::Credentials;

    #[test]
    fn password_hashes_are_salted_and_verifiable() {
        let first = hash_password("correct horse battery staple").unwrap();
        let second = hash_password("correct horse battery staple").unwrap();
        assert_ne!(first, second);
        assert!(verify_password("correct horse battery staple", &first));
        assert!(!verify_password("wrong password", &first));
    }

    #[test]
    fn credentials_are_bounded_and_require_an_email() {
        assert!(validate_credentials(&Credentials {
            email: "person@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
        }));
        assert!(!validate_credentials(&Credentials {
            email: "not-an-email".to_owned(),
            password: "long-enough-password".to_owned(),
        }));
    }
}

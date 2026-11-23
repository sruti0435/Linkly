use crate::{
    auth,
    error::ApiError,
    models::{
        AuthResponse, AuthUser, CachedUrl, CreateUrl, CreatedUrl, Credentials, DailyClicks,
        UrlAnalytics, UrlSummary,
    },
    state::AppState,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Redirect},
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use sqlx::{Error as SqlxError, Row};
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/api/urls", get(list_urls).post(create_url))
        .route("/api/urls/{code}", delete(delete_url))
        .route("/api/urls/{code}/analytics", get(url_analytics))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::authenticate,
        ));

    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/{code}", get(redirect))
        .merge(protected)
        .with_state(state)
}

async fn register(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> Result<impl IntoResponse, ApiError> {
    if !auth::validate_credentials(&credentials) {
        return Err(auth::invalid_credentials_input());
    }

    let email = credentials.email.trim().to_ascii_lowercase();
    let password_hash =
        auth::hash_password(&credentials.password).map_err(auth::internal_auth_error)?;
    let user_id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&email)
        .bind(password_hash)
        .execute(&state.db)
        .await;

    if let Err(error) = result {
        if auth::is_duplicate_key(&error) {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "an account with this email already exists",
            ));
        }
        return Err(auth::internal_auth_error(error));
    }

    auth_response(user_id, email, &state)
}

async fn login(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> Result<impl IntoResponse, ApiError> {
    if !auth::validate_credentials(&credentials) {
        return Err(auth::bad_credentials());
    }

    let email = credentials.email.trim().to_ascii_lowercase();
    let row = sqlx::query("SELECT id, password_hash FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.db)
        .await
        .map_err(auth::internal_auth_error)?;
    let Some(row) = row else {
        return Err(auth::bad_credentials());
    };
    let user_id: Uuid = row.try_get("id").map_err(auth::internal_auth_error)?;
    let password_hash: String = row
        .try_get("password_hash")
        .map_err(auth::internal_auth_error)?;
    if !auth::verify_password(&credentials.password, &password_hash) {
        return Err(auth::bad_credentials());
    }

    auth_response(user_id, email, &state)
}

fn auth_response(
    user_id: Uuid,
    email: String,
    state: &AppState,
) -> Result<Json<AuthResponse>, ApiError> {
    let token =
        auth::create_token(user_id, &state.config.jwt_secret).map_err(auth::internal_auth_error)?;
    Ok(Json(AuthResponse { token, email }))
}

async fn list_urls(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<Vec<UrlSummary>>, ApiError> {
    let urls = sqlx::query_as::<_, UrlSummary>(
        "SELECT u.id, u.original_url, u.short_code, u.created_at, u.expires_at, u.is_active, \
         COUNT(c.id)::BIGINT AS clicks \
         FROM urls u LEFT JOIN clicks c ON c.url_id = u.id \
         WHERE u.user_id = $1 \
         GROUP BY u.id ORDER BY u.created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(log_database_error)?;
    Ok(Json(urls))
}

async fn create_url(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(input): Json<CreateUrl>,
) -> Result<(StatusCode, Json<CreatedUrl>), ApiError> {
    validate_destination(&input.original_url)?;
    if input.expires_at.is_some_and(|date| date <= Utc::now()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "expiration time must be in the future",
        ));
    }
    if let Some(alias) = &input.custom_alias
        && !valid_alias(alias)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "alias must be 3-32 letters, numbers, hyphens, or underscores",
        ));
    }

    let attempts = if input.custom_alias.is_some() { 1 } else { 5 };
    for _ in 0..attempts {
        let code = input
            .custom_alias
            .clone()
            .unwrap_or_else(|| nanoid::nanoid!(8));
        let id = Uuid::new_v4();
        let inserted = sqlx::query_as::<_, (DateTime<Utc>,)>(
            "INSERT INTO urls (id, original_url, short_code, expires_at, user_id) \
             VALUES ($1, $2, $3, $4, $5) RETURNING created_at",
        )
        .bind(id)
        .bind(&input.original_url)
        .bind(&code)
        .bind(input.expires_at)
        .bind(user.id)
        .fetch_one(&state.db)
        .await;

        match inserted {
            Ok((created_at,)) => {
                return Ok((
                    StatusCode::CREATED,
                    Json(CreatedUrl {
                        id,
                        original_url: input.original_url,
                        short_code: code.clone(),
                        short_url: format!("{}/{}", state.config.base_url, code),
                        created_at,
                        expires_at: input.expires_at,
                    }),
                ));
            }
            Err(error) if auth::is_duplicate_key(&error) => {
                if input.custom_alias.is_some() {
                    return Err(ApiError::new(
                        StatusCode::CONFLICT,
                        "that short code is already in use",
                    ));
                }
            }
            Err(error) => return Err(log_database_error(error)),
        }
    }

    Err(ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "could not generate a unique short code; please retry",
    ))
}

async fn delete_url(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(code): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted =
        sqlx::query("DELETE FROM urls WHERE short_code = $1 AND user_id = $2 RETURNING id")
            .bind(&code)
            .bind(user.id)
            .fetch_optional(&state.db)
            .await
            .map_err(log_database_error)?;
    if deleted.is_none() {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "short URL not found"));
    }

    if let Some(cache) = &state.cache {
        match cache.get_multiplexed_async_connection().await {
            Ok(mut connection) => {
                if let Err(error) = connection.del::<_, ()>(cache_key(&code)).await {
                    tracing::warn!(%error, "failed to invalidate deleted URL cache entry");
                }
            }
            Err(error) => {
                tracing::warn!(%error, "Redis unavailable while invalidating deleted URL");
            }
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn url_analytics(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(code): Path<String>,
) -> Result<Json<UrlAnalytics>, ApiError> {
    let url = sqlx::query("SELECT id FROM urls WHERE short_code = $1 AND user_id = $2")
        .bind(&code)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await
        .map_err(log_database_error)?;
    let Some(url) = url else {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "short URL not found"));
    };
    let url_id: Uuid = url.try_get("id").map_err(log_database_error)?;

    let total_clicks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clicks WHERE url_id = $1")
        .bind(url_id)
        .fetch_one(&state.db)
        .await
        .map_err(log_database_error)?;
    let daily_clicks = sqlx::query_as::<_, DailyClicks>(
        "SELECT (clicked_at AT TIME ZONE 'UTC')::DATE AS day, COUNT(*)::BIGINT AS clicks \
         FROM clicks WHERE url_id = $1 \
         GROUP BY day ORDER BY day DESC LIMIT 30",
    )
    .bind(url_id)
    .fetch_all(&state.db)
    .await
    .map_err(log_database_error)?;

    Ok(Json(UrlAnalytics {
        short_code: code,
        total_clicks,
        daily_clicks,
    }))
}

async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Redirect, ApiError> {
    if !valid_alias(&code) {
        return Err(ApiError::new(StatusCode::NOT_FOUND, "short URL not found"));
    }

    let cached = if let Some(cache) = &state.cache {
        match cache.get_multiplexed_async_connection().await {
            Ok(mut connection) => match connection.get::<_, String>(cache_key(&code)).await {
                Ok(serialized) => match serde_json::from_str::<CachedUrl>(&serialized) {
                    Ok(url) => Some(url),
                    Err(error) => {
                        tracing::warn!(%error, "invalid Redis URL cache entry");
                        None
                    }
                },
                Err(error) => {
                    tracing::debug!(%error, "Redis cache miss or unavailable");
                    None
                }
            },
            Err(error) => {
                tracing::debug!(%error, "Redis unavailable; resolving URL from PostgreSQL");
                None
            }
        }
    } else {
        None
    };

    let destination = if let Some(url) = cached {
        if !url.is_active
            || url
                .expires_at
                .is_some_and(|expires_at| expires_at <= Utc::now())
        {
            return Err(ApiError::new(StatusCode::GONE, "short URL has expired"));
        }
        (url.id, url.original_url)
    } else {
        let row = sqlx::query(
            "SELECT id, original_url, expires_at, is_active FROM urls WHERE short_code = $1",
        )
        .bind(&code)
        .fetch_optional(&state.db)
        .await
        .map_err(log_database_error)?;
        let Some(row) = row else {
            return Err(ApiError::new(StatusCode::NOT_FOUND, "short URL not found"));
        };
        let is_active: bool = row.try_get("is_active").map_err(log_database_error)?;
        let expires_at: Option<DateTime<Utc>> =
            row.try_get("expires_at").map_err(log_database_error)?;
        if !is_active || expires_at.is_some_and(|expires_at| expires_at <= Utc::now()) {
            return Err(ApiError::new(StatusCode::GONE, "short URL has expired"));
        }
        let id: Uuid = row.try_get("id").map_err(log_database_error)?;
        let original_url: String = row.try_get("original_url").map_err(log_database_error)?;
        cache_url(
            &state,
            &code,
            &CachedUrl {
                id,
                original_url: original_url.clone(),
                expires_at,
                is_active,
            },
        )
        .await;
        (id, original_url)
    };

    let (url_id, destination) = destination;

    if let Err(error) = sqlx::query("INSERT INTO clicks (url_id) VALUES ($1)")
        .bind(url_id)
        .execute(&state.db)
        .await
    {
        tracing::error!(%error, %code, "failed to record URL click");
    }
    Ok(Redirect::temporary(&destination))
}

async fn cache_url(state: &AppState, code: &str, url: &CachedUrl) {
    let mut ttl = 300_i64;
    if let Some(expires_at) = url.expires_at {
        ttl = ttl.min((expires_at - Utc::now()).num_seconds());
    }
    if ttl <= 0 {
        return;
    }
    let serialized = match serde_json::to_string(url) {
        Ok(serialized) => serialized,
        Err(error) => {
            tracing::error!(%error, "failed to serialize URL cache entry");
            return;
        }
    };
    if let Some(cache) = &state.cache {
        match cache.get_multiplexed_async_connection().await {
            Ok(mut connection) => {
                if let Err(error) = connection
                    .set_ex::<_, _, ()>(cache_key(code), serialized, ttl as u64)
                    .await
                {
                    tracing::warn!(%error, "failed to cache URL mapping");
                }
            }
            Err(error) => tracing::warn!(%error, "Redis unavailable while caching URL mapping"),
        }
    }
}

fn cache_key(code: &str) -> String {
    format!("rusturl:url:{code}")
}

fn validate_destination(value: &str) -> Result<(), ApiError> {
    let valid = url::Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https") && url.host().is_some());
    if !valid {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "destination must be a valid HTTP or HTTPS URL",
        ));
    }
    Ok(())
}

fn valid_alias(alias: &str) -> bool {
    let bytes = alias.as_bytes();
    (3..=32).contains(&bytes.len())
        && (bytes[0].is_ascii_alphanumeric() || matches!(bytes[0], b'-' | b'_'))
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn log_database_error(error: SqlxError) -> ApiError {
    tracing::error!(%error, "database operation failed");
    ApiError::internal()
}

#[cfg(test)]
mod tests {
    use super::{valid_alias, validate_destination};

    #[test]
    fn alias_validation_accepts_only_safe_path_segments() {
        assert!(valid_alias("my-link_2"));
        assert!(!valid_alias("ab"));
        assert!(!valid_alias("../admin"));
        assert!(!valid_alias("with space"));
    }

    #[test]
    fn destination_validation_requires_http_or_https() {
        assert!(validate_destination("https://example.com/a?b=c").is_ok());
        assert!(validate_destination("javascript:alert(1)").is_err());
        assert!(validate_destination("not a url").is_err());
    }
}

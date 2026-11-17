use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[cfg(test)]
use argon2::password_hash::PasswordHasher;
use argon2::{
    Argon2,
    password_hash::{PasswordVerifier, phc::PasswordHash},
};
use axum::{
    Json,
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePool;

use crate::App;
use crate::error::AppError;

const SESSION_COOKIE: &str = "vivec_session";
const SESSION_HOURS: i64 = 12;
const LOGIN_LIMIT: u8 = 5;
const LOGIN_WINDOW: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
pub struct Auth {
    username: Option<String>,
    password_hash: Option<String>,
    secure_cookie: bool,
    attempts: Arc<Mutex<HashMap<String, LoginAttempt>>>,
}

#[derive(Clone, Copy)]
struct LoginAttempt {
    failures: u8,
    blocked_until: Option<Instant>,
}

impl Auth {
    pub fn from_env() -> Self {
        let username = std::env::var("ADMIN_USER").ok();
        let password_hash = std::env::var("ADMIN_PASSWORD_HASH").ok();

        if username.is_none() || password_hash.is_none() {
            tracing::warn!("админка выключена: задайте ADMIN_USER и ADMIN_PASSWORD_HASH");
        }

        let secure_cookie = std::env::var("SESSION_COOKIE_SECURE")
            .map(|value| value == "true" || value == "1")
            .unwrap_or(false);

        Self {
            username,
            password_hash,
            secure_cookie,
            attempts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn credentials(&self) -> Result<(&str, &str), AppError> {
        match (&self.username, &self.password_hash) {
            (Some(username), Some(password_hash)) => {
                Ok((username.as_str(), password_hash.as_str()))
            }
            _ => Err(AppError::Unavailable("админка не настроена")),
        }
    }

    fn check_rate_limit(&self, username: &str) -> Result<(), AppError> {
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|_| AppError::Internal("блокировка лимита входа повреждена".to_string()))?;
        let Some(attempt) = attempts.get_mut(username) else {
            return Ok(());
        };

        if let Some(until) = attempt.blocked_until {
            if Instant::now() < until {
                return Err(AppError::TooManyRequests);
            }
            *attempt = LoginAttempt {
                failures: 0,
                blocked_until: None,
            };
        }
        Ok(())
    }

    fn register_failure(&self, username: &str) {
        let Ok(mut attempts) = self.attempts.lock() else {
            return;
        };
        let attempt = attempts
            .entry(username.to_string())
            .or_insert(LoginAttempt {
                failures: 0,
                blocked_until: None,
            });
        attempt.failures = attempt.failures.saturating_add(1);
        if attempt.failures >= LOGIN_LIMIT {
            attempt.failures = 0;
            attempt.blocked_until = Some(Instant::now() + LOGIN_WINDOW);
        }
    }

    fn register_success(&self, username: &str) {
        if let Ok(mut attempts) = self.attempts.lock() {
            attempts.remove(username);
        }
    }
}

#[derive(Deserialize)]
pub struct LoginInput {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    username: String,
}

#[derive(Serialize)]
pub struct OkResponse {
    ok: bool,
}

pub async fn login(
    State(app): State<App>,
    jar: CookieJar,
    Json(input): Json<LoginInput>,
) -> Result<impl IntoResponse, AppError> {
    validate_login_input(&input)?;
    app.auth.check_rate_limit(&input.username)?;
    let (username, password_hash) = app.auth.credentials()?;

    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| AppError::Internal("ADMIN_PASSWORD_HASH повреждён".to_string()))?;
    let password_ok = Argon2::default()
        .verify_password(input.password.as_bytes(), &parsed_hash)
        .is_ok();

    if input.username != username || !password_ok {
        app.auth.register_failure(&input.username);
        return Err(AppError::Unauthorized);
    }

    app.auth.register_success(&input.username);
    let token = random_token()?;
    let token_hash = hash_token(&token);

    sqlx::query("DELETE FROM sessions WHERE expires_at <= datetime('now')")
        .execute(&app.pool)
        .await?;
    sqlx::query(
        "INSERT INTO sessions (token_hash, expires_at)
         VALUES (?, datetime('now', ?))",
    )
    .bind(token_hash)
    .bind(format!("+{SESSION_HOURS} hours"))
    .execute(&app.pool)
    .await?;

    let cookie = Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(app.auth.secure_cookie)
        .max_age(cookie::time::Duration::hours(SESSION_HOURS))
        .build();
    let response = Json(UserResponse {
        username: username.to_string(),
    });
    Ok((jar.add(cookie), response))
}

pub async fn logout(State(app): State<App>, jar: CookieJar) -> Result<impl IntoResponse, AppError> {
    if let Some(token) = jar.get(SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(hash_token(token.value()))
            .execute(&app.pool)
            .await?;
    }

    let cookie = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(app.auth.secure_cookie)
        .max_age(cookie::time::Duration::ZERO)
        .build();
    Ok((jar.remove(cookie), Json(OkResponse { ok: true })))
}

pub async fn me(State(app): State<App>) -> Result<Json<UserResponse>, AppError> {
    let (username, _) = app.auth.credentials()?;
    Ok(Json(UserResponse {
        username: username.to_string(),
    }))
}

pub async fn require_session(
    State(app): State<App>,
    jar: CookieJar,
    request: Request,
    next: Next,
) -> Response {
    let Some(token) = jar.get(SESSION_COOKIE) else {
        return AppError::Unauthorized.into_response();
    };

    match session_exists(&app.pool, token.value()).await {
        Ok(true) => next.run(request).await,
        Ok(false) => AppError::Unauthorized.into_response(),
        Err(error) => AppError::Db(error).into_response(),
    }
}

pub async fn require_csrf(request: Request, next: Next) -> Response {
    if matches!(request.method().as_str(), "GET" | "HEAD" | "OPTIONS") {
        return next.run(request).await;
    }

    let valid = request
        .headers()
        .get("x-vivec-csrf")
        .and_then(|value| value.to_str().ok())
        == Some("1");
    if valid {
        next.run(request).await
    } else {
        AppError::Forbidden.into_response()
    }
}

async fn session_exists(pool: &SqlitePool, token: &str) -> sqlx::Result<bool> {
    let row = sqlx::query(
        "SELECT 1 FROM sessions
          WHERE token_hash = ? AND expires_at > datetime('now')",
    )
    .bind(hash_token(token))
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

fn validate_login_input(input: &LoginInput) -> Result<(), AppError> {
    if input.username.is_empty() || input.username.len() > 100 {
        return Err(AppError::BadRequest("неверное имя пользователя"));
    }
    if input.password.is_empty() || input.password.len() > 1024 {
        return Err(AppError::BadRequest("неверный пароль"));
    }
    Ok(())
}

fn random_token() -> Result<String, AppError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| AppError::Internal("не удалось создать сессию".to_string()))?;
    Ok(to_hex(&bytes))
}

fn hash_token(token: &str) -> String {
    to_hex(&Sha256::digest(token.as_bytes()))
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
pub fn hash_password(password: &str) -> Result<String, String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{hash_password, hash_token, to_hex};
    use argon2::{
        Argon2,
        password_hash::{PasswordVerifier, phc::PasswordHash},
    };

    #[test]
    fn password_hash_roundtrip() {
        let hash = hash_password("vivec-test-password").unwrap();
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(
            Argon2::default()
                .verify_password(b"vivec-test-password", &parsed)
                .is_ok()
        );
    }

    #[test]
    fn token_hash_is_stable() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
        assert_eq!(to_hex(&[0, 15, 16, 255]), "000f10ff");
    }
}

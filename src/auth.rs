use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
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
    extract::{ConnectInfo, Request, State},
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePool;
use tokio::sync::Semaphore;

use crate::App;
use crate::error::AppError;

const SESSION_COOKIE: &str = "vivec_session";
const SESSION_HOURS: i64 = 12;
const LOGIN_LIMIT_USER: u8 = 5;
const LOGIN_LIMIT_IP: u8 = 20;
const LOGIN_TRACK_LIMIT: usize = 1024;
const LOGIN_WINDOW: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
pub struct Auth {
    username: Option<String>,
    password_hash: Option<String>,
    secure_cookie: bool,
    trust_proxy: bool,
    attempts: Arc<Mutex<LoginLimits>>,
    argon2_slots: Arc<Semaphore>,
}

#[derive(Clone, Copy)]
struct LoginAttempt {
    failures: u8,
    window_started: Instant,
    blocked_until: Option<Instant>,
}

#[derive(Default)]
struct LoginLimits {
    users: HashMap<String, LoginAttempt>,
    ips: HashMap<IpAddr, LoginAttempt>,
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
            .unwrap_or(true);
        let trust_proxy = std::env::var("TRUST_PROXY")
            .map(|value| value == "true" || value == "1")
            .unwrap_or(false);

        Self {
            username,
            password_hash,
            secure_cookie,
            trust_proxy,
            attempts: Arc::new(Mutex::new(LoginLimits::default())),
            argon2_slots: Arc::new(Semaphore::new(1)),
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

    fn check_rate_limit(&self, username: &str, ip: IpAddr) -> Result<(), AppError> {
        let mut limits = self
            .attempts
            .lock()
            .map_err(|_| AppError::Internal("блокировка лимита входа повреждена".to_string()))?;
        let now = Instant::now();
        limits
            .users
            .retain(|_, attempt| attempt_is_active(attempt, now));
        limits
            .ips
            .retain(|_, attempt| attempt_is_active(attempt, now));

        if !limits.users.contains_key(username) && limits.users.len() >= LOGIN_TRACK_LIMIT {
            return Err(AppError::TooManyRequests);
        }
        if !limits.ips.contains_key(&ip) && limits.ips.len() >= LOGIN_TRACK_LIMIT {
            return Err(AppError::TooManyRequests);
        }

        if let Some(attempt) = limits.users.get_mut(username) {
            check_attempt(attempt, LOGIN_LIMIT_USER, now)?;
        }
        if let Some(attempt) = limits.ips.get_mut(&ip) {
            check_attempt(attempt, LOGIN_LIMIT_IP, now)?;
        }
        Ok(())
    }

    fn register_failure(&self, username: &str, ip: IpAddr) {
        let Ok(mut limits) = self.attempts.lock() else {
            return;
        };
        let now = Instant::now();
        update_failure(
            limits
                .users
                .entry(username.to_string())
                .or_insert(new_attempt(now)),
            LOGIN_LIMIT_USER,
            now,
        );
        update_failure(
            limits.ips.entry(ip).or_insert(new_attempt(now)),
            LOGIN_LIMIT_IP,
            now,
        );
    }

    fn register_success(&self, username: &str, ip: IpAddr) {
        if let Ok(mut limits) = self.attempts.lock() {
            limits.users.remove(username);
            limits.ips.remove(&ip);
        }
    }
}

fn new_attempt(now: Instant) -> LoginAttempt {
    LoginAttempt {
        failures: 0,
        window_started: now,
        blocked_until: None,
    }
}

fn attempt_is_active(attempt: &LoginAttempt, now: Instant) -> bool {
    attempt
        .blocked_until
        .map(|until| until > now)
        .unwrap_or(false)
        || now.duration_since(attempt.window_started) < LOGIN_WINDOW
}

fn check_attempt(attempt: &mut LoginAttempt, limit: u8, now: Instant) -> Result<(), AppError> {
    if let Some(until) = attempt.blocked_until {
        if now < until {
            return Err(AppError::TooManyRequests);
        }
        *attempt = new_attempt(now);
    }
    if now.duration_since(attempt.window_started) >= LOGIN_WINDOW {
        *attempt = new_attempt(now);
    }
    if attempt.failures >= limit {
        return Err(AppError::TooManyRequests);
    }
    Ok(())
}

fn update_failure(attempt: &mut LoginAttempt, limit: u8, now: Instant) {
    attempt.failures = attempt.failures.saturating_add(1);
    if attempt.failures >= limit {
        attempt.failures = 0;
        attempt.blocked_until = Some(now + LOGIN_WINDOW);
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
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(input): Json<LoginInput>,
) -> Result<impl IntoResponse, AppError> {
    validate_login_input(&input)?;
    let ip = client_ip(peer, &headers, app.auth.trust_proxy);
    app.auth.check_rate_limit(&input.username, ip)?;
    let (username, password_hash) = app.auth.credentials()?;

    if input.username != username {
        app.auth.register_failure(&input.username, ip);
        return Err(AppError::Unauthorized);
    }

    let permit = app
        .auth
        .argon2_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::TooManyRequests)?;
    let password = input.password;
    let password_hash = password_hash.to_string();
    let password_ok = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let parsed_hash = PasswordHash::new(&password_hash).map_err(|_| ())?;
        Ok::<bool, ()>(
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok(),
        )
    })
    .await
    .map_err(|error| AppError::Internal(format!("argon2 task: {error}")))?
    .map_err(|_| AppError::Internal("ADMIN_PASSWORD_HASH повреждён".to_string()))?;

    if !password_ok {
        app.auth.register_failure(&input.username, ip);
        return Err(AppError::Unauthorized);
    }

    app.auth.register_success(&input.username, ip);
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

fn client_ip(peer: SocketAddr, headers: &HeaderMap, trust_proxy: bool) -> IpAddr {
    if (trust_proxy || peer.ip().is_loopback())
        && let Some(ip) = headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.rsplit(',').next())
            .and_then(|value| value.trim().parse::<IpAddr>().ok())
    {
        return ip;
    }
    peer.ip()
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
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use axum::http::{HeaderMap, HeaderValue};

    use super::{client_ip, hash_password, hash_token, to_hex};
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

    #[test]
    fn proxy_ip_uses_last_forwarded_address() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("198.51.100.1, 203.0.113.7"),
        );
        let peer = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), 1234);
        assert_eq!(
            client_ip(peer, &headers, true),
            "203.0.113.7".parse::<IpAddr>().unwrap()
        );
    }
}

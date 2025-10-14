use std::fmt::Debug;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::error::{AppError, Context, Result};

const AUTH_USER_ENDPOINT: &str = "https://api.jquants.com/v1/token/auth_user";
const AUTH_REFRESH_ENDPOINT: &str = "https://api.jquants.com/v1/token/auth_refresh";
const TOKEN_MARGIN: Duration = Duration::from_secs(120);

struct TokenState {
    id_token: String,
    refresh_token: String,
    expires_at: Instant,
}

static TOKEN_CACHE: OnceLock<Mutex<Option<TokenState>>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct AuthUserResponse {
    #[serde(rename = "refreshToken")]
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct AuthRefreshResponse {
    #[serde(rename = "idToken")]
    id_token: String,
    #[serde(rename = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(rename = "expiresIn")]
    expires_in: Option<String>,
}

fn cache_mutex() -> &'static Mutex<Option<TokenState>> {
    TOKEN_CACHE.get_or_init(|| Mutex::new(None))
}

/// Resolve a placeholder that may come from the environment or require runtime fetching.
pub fn resolve_placeholder(name: &str) -> Result<String> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(_) if name.eq_ignore_ascii_case("JQUANTS_TOKEN") => get_jquants_token(),
        Err(_) => Err(AppError::message(format!(
            "Environment variable {} required by request header is not set",
            name
        ))),
    }
}

fn get_jquants_token() -> Result<String> {
    let mut guard = cache_mutex()
        .lock()
        .map_err(|_| AppError::message("Failed to lock J-Quants token cache"))?;

    let now = Instant::now();

    if let Some(state) = guard.as_ref() {
        if state.expires_at > now + TOKEN_MARGIN {
            return Ok(state.id_token.clone());
        }
    }

    let current_refresh = guard.as_ref().map(|state| state.refresh_token.clone());

    let new_state = match try_refresh(current_refresh.as_deref()) {
        Ok(state) => state,
        Err(err) => {
            log::warn!("Failed to refresh J-Quants token: {}", err);
            let refreshed = authenticate_from_credentials()?;
            try_refresh(Some(&refreshed.refresh_token))?
        }
    };

    *guard = Some(new_state.clone());
    Ok(new_state.id_token)
}

fn try_refresh(refresh_token: Option<&str>) -> Result<TokenState> {
    let token = refresh_token.ok_or_else(|| AppError::message("Missing J-Quants refresh token"))?;
    refresh_with_token(token)
}

fn authenticate_from_credentials() -> Result<AuthUserResponse> {
    let email = std::env::var("JQUANTS_EMAIL").with_context(|| "Missing JQUANTS_EMAIL env var")?;
    let password =
        std::env::var("JQUANTS_PASSWORD").with_context(|| "Missing JQUANTS_PASSWORD env var")?;

    let client = build_blocking_client()?;

    let response = client
        .post(AUTH_USER_ENDPOINT)
        .json(&serde_json::json!({
            "mailaddress": email,
            "password": password,
        }))
        .send()
        .context("Failed to request J-Quants user authentication")?
        .error_for_status()
        .context("J-Quants user authentication request failed")?;

    let parsed: AuthUserResponse = response
        .json()
        .context("Failed to parse J-Quants auth_user response")?;

    Ok(parsed)
}

fn refresh_with_token(refresh_token: &str) -> Result<TokenState> {
    let client = build_blocking_client()?;

    let response = client
        .post(AUTH_REFRESH_ENDPOINT)
        .json(&serde_json::json!({
            "refreshToken": refresh_token,
        }))
        .send()
        .context("Failed to request J-Quants token refresh")?
        .error_for_status()
        .context("J-Quants token refresh request failed")?;

    let parsed: AuthRefreshResponse = response
        .json()
        .context("Failed to parse J-Quants auth_refresh response")?;

    let expires_in_secs: u64 = parsed
        .expires_in
        .as_deref()
        .unwrap_or("3600")
        .parse()
        .unwrap_or(3600);

    let expires_at = Instant::now() + Duration::from_secs(expires_in_secs);
    let refreshed = TokenState {
        id_token: parsed.id_token,
        refresh_token: parsed
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string()),
        expires_at,
    };

    Ok(refreshed)
}

fn build_blocking_client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to construct blocking HTTP client")?)
}

impl Clone for TokenState {
    fn clone(&self) -> Self {
        Self {
            id_token: self.id_token.clone(),
            refresh_token: self.refresh_token.clone(),
            expires_at: self.expires_at,
        }
    }
}

impl Debug for TokenState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenState")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

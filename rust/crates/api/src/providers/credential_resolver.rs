//! Credential resolution chain for all supported providers.
//!
//! Resolution order (stops at first match):
//!
//! 1. `LLM_AUTH_METHOD=apikey`  → read key from env, fail if absent
//! 2. `LLM_AUTH_METHOD=oauth`   → load saved OAuth token, fail if absent/expired
//! 3. No `LLM_AUTH_METHOD` set  →
//!    a. Provider-specific API key env var (e.g. `ANTHROPIC_API_KEY`)
//!    b. `LLM_API_KEY` generic fallback
//!    c. Saved OAuth token (if present and not expired)
//!    d. Interactive prompt (TTY only)
//!    e. Clear error with actionable message

use crate::error::ApiError;
use crate::providers::credential::{Credential, CredentialSource};
use crate::providers::ProviderKind;

/// Everything needed to resolve credentials for one provider.
pub struct ProviderCredentialConfig {
    pub provider_name:   &'static str,
    /// Primary env var for an API key (None = auth not required).
    pub api_key_env:     Option<&'static str>,
    /// Generic fallback always checked second.
    pub generic_key_env: &'static str,
    /// OAuth token env var fallback (e.g. OPENAI_OAUTH).
    pub oauth_env: Option<&'static str>,
    /// True when no credential is required (Ollama, local stacks).
    pub auth_optional:   bool,
}

impl ProviderCredentialConfig {
    #[must_use]
    pub fn for_kind(kind: ProviderKind) -> Self {
        match kind {
            ProviderKind::Anthropic => Self {
                provider_name:   "Anthropic",
                api_key_env:     Some("ANTHROPIC_API_KEY"),
                generic_key_env: "LLM_API_KEY",
                oauth_env:       None,
                auth_optional:   false,
            },
            ProviderKind::OpenAi => Self {
                provider_name:   "OpenAI",
                api_key_env:     Some("OPENAI_API_KEY"),
                oauth_env:       Some("OPENAI_OAUTH"),
                generic_key_env: "LLM_API_KEY",
                auth_optional:   false,
            },
            ProviderKind::Xai => Self {
                provider_name:   "xAI",
                api_key_env:     Some("XAI_API_KEY"),
                generic_key_env: "LLM_API_KEY",
                oauth_env:       None,
                auth_optional:   false,
            },
            ProviderKind::Ollama | ProviderKind::Generic => Self {
                provider_name:   "local",
                api_key_env:     None,
                generic_key_env: "LLM_API_KEY",
                oauth_env:       None,
                auth_optional:   true,
            },
        }
    }
}

/// Synchronous resolution chain — reads env vars and saved OAuth tokens.
/// Interactive fallback is attempted only when a TTY is attached.
///
/// # Errors
/// Returns `ApiError::MissingCredentials` when no credential can be found
/// and `auth_optional` is false.
pub fn resolve_credential_from_env(
    config: &ProviderCredentialConfig,
) -> Result<(Credential, CredentialSource), ApiError> {
    // Optional providers (Ollama, generic local)
    if config.auth_optional {
        if let Some(key) = optional_key_from_env(config) {
            return Ok((Credential::ApiKey(key), CredentialSource::EnvVar));
        }
        return Ok((Credential::None, CredentialSource::NotRequired));
    }

    // Expert override: explicit auth method
    match std::env::var("LLM_AUTH_METHOD").as_deref() {
        Ok("apikey") => {
            let key = required_key_from_env(config)?;
            return Ok((Credential::ApiKey(key), CredentialSource::EnvVar));
        }
        Ok("oauth") => {
            let token = load_saved_oauth(config.provider_name)?;
            return Ok((Credential::BearerToken(token), CredentialSource::SavedOAuth));
        }
        _ => {}
    }

    // 3a. Provider-specific API key env var
    if let Some(key) = optional_key_from_env(config) {
        return Ok((Credential::ApiKey(key), CredentialSource::EnvVar));
    }

    // 3a2. OAuth env var fallback (e.g. OPENAI_OAUTH, OPENCLAW_TOKEN)
    for oauth_var in [Some("OPENCLAW_GATEWAY_TOKEN"), config.oauth_env].iter().flatten() {
        if let Ok(token) = std::env::var(oauth_var) {
            if !token.is_empty() {
                return Ok((Credential::BearerToken(token), CredentialSource::EnvVar));
            }
        }
    }

    // 3b. Generic LLM_API_KEY
    if let Ok(key) = std::env::var("LLM_API_KEY") {
        if !key.is_empty() {
            return Ok((Credential::ApiKey(key), CredentialSource::EnvVar));
        }
    }

    // 3c. Saved OAuth token
    if let Ok(token) = load_saved_oauth(config.provider_name) {
        return Ok((Credential::BearerToken(token), CredentialSource::SavedOAuth));
    }

    // 3d. Clear error
    Err(missing_credentials_error(config))
}

/// Build a MissingCredentials error using only 'static slices.
fn missing_credentials_error(config: &ProviderCredentialConfig) -> ApiError {
    match config.api_key_env {
        Some("ANTHROPIC_API_KEY") => ApiError::missing_credentials(
            config.provider_name,
            &["ANTHROPIC_API_KEY", "LLM_API_KEY"],
        ),
        Some("OPENAI_API_KEY") => ApiError::missing_credentials(
            config.provider_name,
            &["OPENAI_API_KEY", "OPENAI_OAUTH", "LLM_API_KEY"],
        ),
        Some("XAI_API_KEY") => ApiError::missing_credentials(
            config.provider_name,
            &["XAI_API_KEY", "LLM_API_KEY"],
        ),
        _ => ApiError::missing_credentials(config.provider_name, &["LLM_API_KEY"]),
    }
}

fn optional_key_from_env(config: &ProviderCredentialConfig) -> Option<String> {
    config
        .api_key_env
        .and_then(|var| std::env::var(var).ok())
        .filter(|k| !k.is_empty())
}

fn required_key_from_env(config: &ProviderCredentialConfig) -> Result<String, ApiError> {
    optional_key_from_env(config)
        .or_else(|| std::env::var("LLM_API_KEY").ok().filter(|k| !k.is_empty()))
        .ok_or_else(|| missing_credentials_error(config))
}

/// Load a saved OAuth access token for the given provider from disk.
/// Token files are stored at `~/.config/claw/{provider}_oauth.json`.
fn load_saved_oauth(provider_name: &str) -> Result<String, ApiError> {
    let key = provider_name.to_lowercase().replace(' ', "_");
    let path = dirs_path().join(format!("{key}_oauth.json"));
    if !path.exists() {
        return Err(ApiError::Auth(format!(
            "no saved OAuth token found for {provider_name}"
        )));
    }
    let text = std::fs::read_to_string(&path).map_err(ApiError::Io)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(ApiError::from)?;
    // Check expiry
    if let Some(expires_at) = parsed["expires_at"].as_u64() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        if now > expires_at && parsed["refresh_token"].is_null() {
            return Err(ApiError::Auth(format!(
                "saved OAuth token for {provider_name} has expired; run `claw login`"
            )));
        }
    }
    parsed["access_token"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| ApiError::Auth(format!("malformed OAuth token file for {provider_name}")))
}

fn dirs_path() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join(".config/claw")
}

/// Minimal interactive key prompt — only reached when a TTY is attached
/// and no env var or saved token was found.
fn prompt_for_key(provider_name: &str) -> Result<String, ApiError> {
    use std::io::Write;
    eprint!(
        "\n[{provider_name}] No credentials found. Paste your API key (or Ctrl-C to abort): "
    );
    std::io::stderr().flush().ok();
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(ApiError::Io)?;
    let key = input.trim().to_string();
    if key.is_empty() {
        return Err(ApiError::Auth("no key entered".to_string()));
    }
    Ok(key)
}

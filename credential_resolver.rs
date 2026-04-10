/// This is the resolution chain. 
/// Each provider gets a ProviderCredentialConfig that describes where to look 
/// and what the OAuth endpoints are. The resolver tries sources in order.

use crate::error::ApiError;
use crate::providers::credential::{looks_like_api_key, Credential, CredentialSource};
use crate::providers::claw_provider::{oauth_token_is_expired, resolve_saved_oauth_token};
use crate::providers::ProviderKind;

/// Everything needed to resolve credentials for one provider.
pub struct ProviderCredentialConfig {
    pub provider_name:     &'static str,
    /// Primary env var for an API key (e.g. "ANTHROPIC_API_KEY").
    pub api_key_env:       Option<&'static str>,
    /// Generic fallback env var always checked second.
    pub generic_key_env:   &'static str,   // "LLM_API_KEY"
    /// Where saved OAuth tokens are stored on disk (provider-scoped).
    pub oauth_token_path:  Option<&'static str>,
    /// True when no credential is required (Ollama, local stacks).
    pub auth_optional:     bool,
}

impl ProviderCredentialConfig {
    pub fn for_kind(kind: ProviderKind) -> Self {
        match kind {
            ProviderKind::ClawApi => Self {
                provider_name:    "Anthropic",
                api_key_env:      Some("ANTHROPIC_API_KEY"),
                generic_key_env:  "LLM_API_KEY",
                oauth_token_path: Some("anthropic"),
                auth_optional:    false,
            },
            ProviderKind::OpenAi => Self {
                provider_name:    "OpenAI",
                api_key_env:      Some("OPENAI_API_KEY"),
                generic_key_env:  "LLM_API_KEY",
                oauth_token_path: Some("openai"),
                auth_optional:    false,
            },
            ProviderKind::Xai => Self {
                provider_name:    "xAI",
                api_key_env:      Some("XAI_API_KEY"),
                generic_key_env:  "LLM_API_KEY",
                oauth_token_path: None,  // xAI has no public OAuth yet
                auth_optional:    false,
            },
            ProviderKind::Ollama | ProviderKind::Generic => Self {
                provider_name:    "local",
                api_key_env:      None,
                generic_key_env:  "LLM_API_KEY",
                oauth_token_path: None,
                auth_optional:    true,
            },
        }
    }
}

/// Resolution chain (in order):
///
/// 1. `LLM_AUTH_METHOD=apikey`  → read key from env var, fail if absent
/// 2. `LLM_AUTH_METHOD=oauth`   → load saved OAuth token, fail if absent/expired
/// 3. No `LLM_AUTH_METHOD` set  →
///    a. provider-specific API key env var
///    b. LLM_API_KEY generic env var
///    c. saved OAuth token (if not expired)
///    d. interactive prompt (if a TTY is attached)
///    e. error with a clear human-readable message
pub async fn resolve_credential(
    config: &ProviderCredentialConfig,
    allow_interactive: bool,
) -> Result<(Credential, CredentialSource), ApiError> {

    if config.auth_optional {
        // Check if a key was voluntarily provided anyway (e.g. protected Ollama)
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
            let token = load_saved_oauth(config)?;
            return Ok((Credential::OAuth(token), CredentialSource::SavedOAuth));
        }
        _ => {}
    }

    // 3a. Provider-specific API key env var
    if let Some(key) = optional_key_from_env(config) {
        return Ok((Credential::ApiKey(key), CredentialSource::EnvVar));
    }

    // 3b. Generic LLM_API_KEY
    if let Ok(key) = std::env::var("LLM_API_KEY") {
        if !key.is_empty() {
            return Ok((Credential::ApiKey(key), CredentialSource::EnvVar));
        }
    }

    // 3c. Saved OAuth token
    if let Ok(token) = load_saved_oauth(config) {
        if !oauth_token_is_expired(&token) {
            return Ok((Credential::OAuth(token), CredentialSource::SavedOAuth));
        }
        // Expired but has refresh → caller should refresh, not re-login
        if token.refresh_token.is_some() {
            return Ok((Credential::OAuth(token), CredentialSource::SavedOAuth));
        }
        // Expired, no refresh → fall through to interactive
    }

    // 3d. Interactive prompt (TTY only)
    if allow_interactive && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        return resolve_interactive(config).await;
    }

    // 3e. Clear error
    Err(ApiError::missing_credentials(
        config.provider_name,
        &[
            config.api_key_env.unwrap_or("LLM_API_KEY"),
            "LLM_API_KEY",
        ],
    ))
}

fn optional_key_from_env(config: &ProviderCredentialConfig) -> Option<String> {
    config.api_key_env
        .and_then(|var| std::env::var(var).ok())
        .filter(|k| !k.is_empty())
}

fn required_key_from_env(config: &ProviderCredentialConfig) -> Result<String, ApiError> {
    optional_key_from_env(config)
        .or_else(|| std::env::var("LLM_API_KEY").ok().filter(|k| !k.is_empty()))
        .ok_or_else(|| ApiError::missing_credentials(
            config.provider_name,
            &[config.api_key_env.unwrap_or("LLM_API_KEY"), "LLM_API_KEY"],
        ))
}

fn load_saved_oauth(
    config: &ProviderCredentialConfig,
) -> Result<crate::providers::claw_provider::OAuthTokenSet, ApiError> {
    let scope = config.oauth_token_path
        .ok_or_else(|| ApiError::Auth(format!(
            "{} does not support OAuth login", config.provider_name
        )))?;
    resolve_saved_oauth_token(scope)
}

/// Interactive credential prompt — the user-friendly path.
/// Naive users never need to understand API keys vs OAuth; they just type
/// what they have. The function sniffs the input and routes accordingly.
async fn resolve_interactive(
    config: &ProviderCredentialConfig,
) -> Result<(Credential, CredentialSource), ApiError> {
    use std::io::Write;

    eprintln!(
        "\n[{}] No credentials found. You have two options:",
        config.provider_name
    );
    eprintln!("  1) Log in with your {} account (browser-based)", config.provider_name);
    eprintln!("  2) Paste an API key (from your account's API settings page)");
    eprint!("\nEnter choice [1/2], or paste your key directly: ");
    std::io::stderr().flush().ok();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).map_err(ApiError::Io)?;
    let input = input.trim();

    match input {
        "1" => {
            // Delegate to the existing OAuth login flow
            eprintln!("Opening browser for {} login...", config.provider_name);
            let token = crate::providers::claw_provider::run_oauth_login(
                config.oauth_token_path.unwrap_or("generic"),
                config.provider_name,
            ).await?;
            Ok((Credential::OAuth(token), CredentialSource::Interactive))
        }
        "2" => {
            eprint!("Paste your API key: ");
            std::io::stderr().flush().ok();
            let mut key = String::new();
            std::io::stdin().read_line(&mut key).map_err(ApiError::Io)?;
            let key = key.trim().to_string();
            if key.is_empty() {
                return Err(ApiError::Auth("No key entered.".to_string()));
            }
            Ok((Credential::ApiKey(key), CredentialSource::Interactive))
        }
        other if looks_like_api_key(other, config.provider_name) => {
            // User directly pasted a key — treat it as option 2
            Ok((Credential::ApiKey(other.to_string()), CredentialSource::Interactive))
        }
        _ => Err(ApiError::Auth(
            "Unrecognised input. Run `claw login` to authenticate.".to_string(),
        )),
    }
}
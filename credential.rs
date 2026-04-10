use std::fmt;
use crate::providers::claw_provider::OAuthTokenSet;

/// A resolved credential — either a bearer/API key or a live OAuth token.
/// The rest of the codebase only ever sees this type; it never needs to know
/// whether the user configured an API key or went through an OAuth flow.
#[derive(Debug, Clone)]
pub enum Credential {
    /// A static API key or personal access token.
    ApiKey(String),
    /// A live OAuth token set (access + optional refresh).
    OAuth(OAuthTokenSet),
    /// No credential required (e.g. Ollama running locally).
    None,
}

impl Credential {
    /// Returns the value to use as a Bearer token in Authorization headers,
    /// or None if no auth header should be sent.
    pub fn bearer_token(&self) -> Option<&str> {
        match self {
            Self::ApiKey(key)  => Some(key.as_str()),
            Self::OAuth(token) => Some(token.access_token.as_str()),
            Self::None         => None,
        }
    }

    /// True if the credential is an OAuth token that has expired and
    /// cannot be refreshed. Callers should re-run the login flow.
    pub fn is_expired(&self) -> bool {
        match self {
            Self::OAuth(token) => crate::client::oauth_token_is_expired(token)
                && token.refresh_token.is_none(),
            _ => false,
        }
    }
}

/// Describes how an API key looks — used to distinguish a pasted API key
/// from a mistakenly entered OAuth token or password in the interactive prompt.
pub fn looks_like_api_key(s: &str, provider: &str) -> bool {
    match provider {
        "anthropic" => s.starts_with("sk-ant-"),
        "openai"    => s.starts_with("sk-"),
        "xai"       => s.starts_with("xai-"),
        _           => s.len() > 20 && !s.contains(' '),
    }
}

/// Human-readable credential source, for diagnostic output only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    EnvVar,
    SavedOAuth,
    Interactive,
    NotRequired,
}

impl fmt::Display for CredentialSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvVar      => write!(f, "environment variable"),
            Self::SavedOAuth  => write!(f, "saved login session"),
            Self::Interactive => write!(f, "interactive prompt"),
            Self::NotRequired => write!(f, "not required"),
        }
    }
}
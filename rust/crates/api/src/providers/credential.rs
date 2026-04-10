use std::fmt;

/// A resolved credential — either a static key, an OAuth bearer token,
/// or nothing (for local providers like Ollama that need no auth).
///
/// The rest of the codebase only ever sees this type; it never needs to know
/// whether the user configured an API key or went through an OAuth flow.
#[derive(Debug, Clone)]
pub enum Credential {
    /// A static API key or personal access token.
    ApiKey(String),
    /// A live OAuth bearer token (e.g. from `claw login`).
    BearerToken(String),
    /// No credential required (e.g. Ollama running locally).
    None,
}

impl Credential {
    /// Returns the value to use as an auth token in request headers,
    /// or None if no auth header should be sent.
    #[must_use]
    pub fn bearer_token(&self) -> Option<&str> {
        match self {
            Self::ApiKey(key)      => Some(key.as_str()),
            Self::BearerToken(tok) => Some(tok.as_str()),
            Self::None             => None,
        }
    }

    /// True when no credential is available and none is required.
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// Heuristic to distinguish a pasted API key from other input.
/// Used in the interactive prompt to decide how to treat user input.
#[must_use]
pub fn looks_like_api_key(s: &str, provider: &str) -> bool {
    match provider {
        "anthropic" => s.starts_with("sk-ant-"),
        "openai"    => s.starts_with("sk-"),
        "xai"       => s.starts_with("xai-"),
        _           => s.len() > 20 && !s.contains(' '),
    }
}

/// Human-readable credential source — for diagnostic output only.
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

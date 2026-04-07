from __future__ import annotations
import getpass
import os
import sys


def _resolve_provider() -> str:
    if p := os.environ.get("LLM_PROVIDER"):
        return p.lower()
    model = os.environ.get("LLM_MODEL", "")
    if model.startswith("claude"):
        return "anthropic"
    if model.startswith("grok"):
        return "xai"
    if model.startswith(("gpt", "o1", "o3")):
        return "openai"
    if os.environ.get("LLM_BASE_URL"):
        return "generic"
    return "anthropic"


def _base_url(provider: str) -> str:
    if url := os.environ.get("LLM_BASE_URL"):
        return url.rstrip("/")
    return {
        "anthropic": "https://api.anthropic.com",
        "openai":    "https://api.openai.com",
        "xai":       "https://api.x.ai",
        "ollama":    "http://localhost:11434",
    }.get(provider, "http://localhost:11434")


def _resolve_credential(provider: str) -> str | None:
    if provider in ("ollama", "generic"):
        return os.environ.get("LLM_API_KEY") or None

    method = os.environ.get("LLM_AUTH_METHOD", "")
    candidates = {
        "anthropic": ["ANTHROPIC_API_KEY", "LLM_API_KEY"],
        "openai":    ["OPENAI_API_KEY",    "LLM_API_KEY"],
        "xai":       ["XAI_API_KEY",       "LLM_API_KEY"],
    }.get(provider, ["LLM_API_KEY"])

    if method == "apikey":
        for var in candidates:
            if v := os.environ.get(var):
                return v
        raise EnvironmentError(
            f"LLM_AUTH_METHOD=apikey but no API key env var found for {provider}"
        )

    for var in candidates:
        if v := os.environ.get(var):
            return v

    if token := _load_saved_oauth(provider, silent=True):
        return token

    if sys.stdin.isatty():
        return _interactive_prompt(provider)

    raise EnvironmentError(
        f"No credentials for {provider}. Set {candidates[0]} or run `claw login`."
    )


def _load_saved_oauth(provider: str, silent: bool = False) -> str | None:
    import json
    import pathlib
    import time

    token_file = pathlib.Path.home() / ".config" / "claw" / f"{provider}_oauth.json"
    if not token_file.exists():
        return None
    data = json.loads(token_file.read_text())
    expires_at = data.get("expires_at", 0)
    if expires_at and time.time() > expires_at and not data.get("refresh_token"):
        if not silent:
            print(
                f"Saved {provider} OAuth token has expired. "
                f"Run `claw login --provider {provider}`."
            )
        return None
    return data.get("access_token")


def _interactive_prompt(provider: str) -> str:
    print(f"\n[{provider}] No credentials found.")
    print("  1) Log in with your account (browser-based)")
    print("  2) Paste your API key")
    choice = input("Choice [1/2], or paste key directly: ").strip()
    if choice == "1":
        raise NotImplementedError(
            "Run `claw login` from the CLI for browser-based login."
        )
    if choice == "2" or len(choice) > 20:
        return choice if len(choice) > 20 else getpass.getpass("API key: ").strip()
    raise ValueError("No credential provided.")


def _headers(provider: str) -> dict[str, str]:
    headers = {"Content-Type": "application/json"}
    key = _resolve_credential(provider)
    if provider == "anthropic" and key:
        headers["x-api-key"] = key
        headers["anthropic-version"] = "2023-06-01"
    elif key:
        headers["Authorization"] = f"Bearer {key}"
    return headers


def complete(messages: list[dict], **kw) -> str:
    """One-shot completion. Returns the assistant message text."""
    import httpx

    provider = _resolve_provider()
    model    = os.environ.get("LLM_MODEL", "claude-sonnet-4-6")
    base     = _base_url(provider)

    if provider == "anthropic":
        url     = f"{base}/v1/messages"
        payload = {
            "model": model,
            "max_tokens": kw.pop("max_tokens", 4096),
            "messages": messages,
            **kw,
        }
    else:
        url     = f"{base}/v1/chat/completions"
        payload = {"model": model, "messages": messages, **kw}

    resp = httpx.post(url, json=payload, headers=_headers(provider), timeout=120)
    resp.raise_for_status()
    data = resp.json()

    if provider == "anthropic":
        return data["content"][0]["text"]
    return data["choices"][0]["message"]["content"]

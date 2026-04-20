# How to use claw with multiple providers

## Prerequisites

claw speaks OpenAI-compatible API format natively. Any provider that exposes
that format works directly — no LiteLLM or other proxy needed.

## Ollama (local, no auth)

```bash
claw --provider ollama --model qwen3-coder:480b-cloud
```

Requires Ollama running locally. Default base URL: `http://localhost:11434/v1`.
Override with `OLLAMA_BASE_URL`.

## OpenAI via OpenClaw gateway (OAuth)

```bash
export OPENCLAW_GATEWAY_URL=http://localhost:18789
export OPENCLAW_GATEWAY_TOKEN=<your-gateway-token>
claw --provider openai --model gpt-5.4
```

claw routes through OpenClaw's OpenAI-compatible gateway endpoint, passing
the actual model via `x-openclaw-model` header. OpenClaw handles OAuth.

## Setting provider defaults

Add to `~/.claw.json`:

```json
{
  "providerDefaults": {
    "openai": { "defaultModel": "gpt-5.4" },
    "ollama": { "defaultModel": "qwen3-coder:480b-cloud" }
  }
}
```

Then `claw --provider openai` uses the configured default without `--model`.

## Provider env vars

| Provider | Auth env var | Base URL env var |
|---|---|---|
| Ollama | none | `OLLAMA_BASE_URL` |
| OpenAI (direct) | `OPENAI_API_KEY` | `OPENAI_BASE_URL` |
| OpenAI (via OpenClaw) | `OPENCLAW_GATEWAY_TOKEN` | `OPENCLAW_GATEWAY_URL` |
| OpenAI (OAuth) | `OPENAI_OAUTH` | — |
| xAI | `XAI_API_KEY` | `XAI_BASE_URL` |
| Generic | `LLM_API_KEY` | `LLM_BASE_URL` |

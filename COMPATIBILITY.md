# Provider Compatibility Matrix

Branch: `Ch00rD/claw-code:main`
Last updated: 2026-04-10

## How to run

```bash
# Required for non-Anthropic providers
export LLM_API_KEY=dummy         # or real key for paid providers
unset ANTHROPIC_BASE_URL LLM_BASE_URL  # ensure no LiteLLM routing

# Launch with explicit provider
claw --provider ollama --model "qwen3-coder:480b-cloud"
# or shorthand
claw --model "ollama/qwen3-coder:480b-cloud"
```

## Matrix

| Provider | Model | Base URL | Key required | Command | Result | Notes |
|---|---|---|---|---|---|---|
| Ollama | qwen3-coder:480b-cloud | <http://localhost:11434/v1> | No | `claw --model ollama/qwen3-coder:480b-cloud` | ✅ Works | Direct, no LiteLLM needed |
| Ollama | minimax-m2.7:cloud | <http://localhost:11434/v1> | No | `claw --model ollama/minimax-m2.7:cloud` | 🔲 Untested | |
| Ollama | deepseek-coder:1.3b | <http://localhost:11434/v1> | No | `claw --model ollama/deepseek-coder:1.3b` | 🔲 Untested | |
| OpenAI | gpt-4o | <https://api.openai.com/v1> | Yes (OPENAI_API_KEY) | `claw --model openai/gpt-4o` | 🔲 Untested | |
| xAI | grok-3 | <https://api.x.ai/v1> | Yes (XAI_API_KEY) | `claw --model grok-3` | 🔲 Untested | Auto-detected by model prefix |
| DashScope | qwen-plus | <https://dashscope.aliyuncs.com/compatible-mode/v1> | Yes (DASHSCOPE_API_KEY) | `claw --model qwen-plus` | 🔲 Untested | Auto-detected by qwen- prefix |
| Venice AI | venice-uncensored | <https://api.venice.ai/api/v1> | Yes (LLM_API_KEY) | `claw --provider generic --model venice-uncensored` | 🔲 Untested | |
| Anthropic | claude-opus-4-6 | <https://api.anthropic.com> | Yes (ANTHROPIC_API_KEY or OAuth) | `claw` | 🔲 Untested (no API key) | Legacy path unchanged |

## Known issues

- Model self-identifies as Claude regardless of actual provider (training data issue, system prompt override planned)
- `LLM_BASE_URL` or `ANTHROPIC_BASE_URL` must be unset for direct-to-provider routing to work
- Tests must be run with `LLM_BASE_URL`, `LLM_API_KEY`, `LLM_PROVIDER` unset

## Auth env var precedence (per provider)

| Provider | Primary | Fallback |
|---|---|---|
| Anthropic | ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN | — |
| OpenAI | OPENAI_API_KEY | LLM_API_KEY |
| xAI | XAI_API_KEY | LLM_API_KEY |
| DashScope | DASHSCOPE_API_KEY | LLM_API_KEY |
| Ollama | — (no auth) | LLM_API_KEY (optional) |
| Generic | LLM_API_KEY | — |

## Base URL env var per provider

| Provider | Env var | Default |
|---|---|---|
| Anthropic | ANTHROPIC_BASE_URL | <https://api.anthropic.com> |
| OpenAI | OPENAI_BASE_URL | <https://api.openai.com/v1> |
| xAI | XAI_BASE_URL | <https://api.x.ai/v1> |
| DashScope | DASHSCOPE_BASE_URL | <https://dashscope.aliyuncs.com/compatible-mode/v1> |
| Ollama | OLLAMA_BASE_URL | <http://localhost:11434/v1> |
| Generic | LLM_BASE_URL | <http://localhost:11434/v1> |

# Instructions on how to structure this file from upstream dev team of `claw-code`

Yes — that is a good workflow, and the lowest-noise version is:

1. keep a single compatibility matrix doc in your branch/repo
2. update that same file as you test more providers
3. drop one link here when it is first useful
4. only repost when there is a meaningful delta

- new provider family
- auth behavior changed
- a fallback/regression was confirmed
- a previously-broken route now works

The key thing is: one living document beats many scattered Discord updates.

Best shape for the matrix:

- provider / endpoint
- model used
- base URL form
- key required? yes/no
- exact launch command
- result: works / partial / fails
- failure mode
- notes

If you want maximum maintainer usefulness, keep the entries repro-first, for example:

```text
Provider: Ollama
Model: qwen3-coder:480b-cloud
Base URL: http://127.0.0.1:11434/v1
Command: claw --model ollama/qwen3-coder:480b-cloud
Result: Works
Notes: no Anthropic fallback
```

Even better: when you find a real failure, link to the matrix row and post just the pinpoint in chat:

- expected
- actual
- command/config
- exact error

That gives us both:

- a durable compatibility surface
- and small mergeable/debuggable reports

So short version: **yes, keep it in-branch, make it a living doc, and share the same link instead of fragmenting updates**.
